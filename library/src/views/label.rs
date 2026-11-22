use std::{any::Any, rc::Rc};

use glyphon::{Attrs, Buffer, Family, Metrics};

use crate::{
    backend::{layout_context::LayoutContext, render_context::RenderContext},
    reactive::create_effect,
    types::TextEntry,
    view::{
        IntoView, View,
        update::{StyleChanged, push_update},
        view_id::ViewId,
    },
};

/// Dynamic text. `f` runs in an effect, so signals read inside it keep the
/// label up to date: `label(move || title.get())`.
pub fn label<S: Into<String>>(f: impl Fn() -> S + 'static) -> Label {
    let l = Label::new(String::new());
    let id = l.id;
    create_effect(move || {
        let s: String = f().into();
        push_update(id, Box::new(s));
    });
    l
}

/// Static text.
pub fn text<S: Into<String>>(label: S) -> Label {
    Label::new(label)
}

pub struct Label {
    id: ViewId,
    text: String,
    text_entry: Option<Rc<TextEntry>>,
}

impl Label {
    fn build_entry(&self, ctx: &mut LayoutContext, x: f32, y: f32) -> Rc<TextEntry> {
        let style = self.id.state().borrow().style.clone();
        let font_system = &mut ctx.storage_mut().font_system;

        let metrics = Metrics::new(style.font_size, style.font_size * style.line_height);
        let attrs = Attrs::new()
            .family(Family::Name(&style.font_family))
            .weight(style.weight)
            .color(style.text_color.into())
            .style(style.text_style);
        let mut buffer = Buffer::new(font_system, metrics);
        buffer.set_text(
            font_system,
            &self.text,
            &attrs,
            glyphon::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(font_system, false);

        Rc::new(TextEntry {
            buffer,
            x,
            y,
            color: style.text_color.into(),
        })
    }
}

impl View for Label {
    fn id(&self) -> ViewId {
        self.id
    }

    fn update(&mut self, state: Box<dyn Any>) {
        let state = match state.downcast::<String>() {
            Ok(new_text) => {
                if *new_text != self.text {
                    self.text = *new_text;
                    self.text_entry = None; // rebuilt by `layout`
                }
                return;
            }
            Err(other) => other,
        };
        if state.is::<StyleChanged>() {
            self.text_entry = None; // font, size, color or position changed
        }
    }

    // in impl View for Label:
    fn measure(
        &mut self,
        ctx: &mut LayoutContext,
        _known: taffy::Size<Option<f32>>,
        _available: taffy::Size<taffy::AvailableSpace>,
    ) -> Option<taffy::Size<f32>> {
        if self.text_entry.is_none() {
            self.text_entry = Some(self.build_entry(ctx, 0.0, 0.0));
        }
        let buf = &self.text_entry.as_ref()?.buffer;
        let (mut w, mut h) = (0.0f32, 0.0f32);
        for run in buf.layout_runs() {
            w = w.max(run.line_w);
            h = h.max(run.line_top + run.line_height);
        }
        if h == 0.0 {
            h = buf.metrics().line_height; // empty text still takes a line
        }
        Some(taffy::Size {
            width: w.ceil(),
            height: h.ceil(),
        })
    }

    // replaces the old `layout`:
    fn layout(&mut self, ctx: &mut LayoutContext) {
        let r = self.id.rect();
        if self.text_entry.is_none() {
            self.text_entry = Some(self.build_entry(ctx, r.x, r.y));
            return;
        }
        // The previous frame's draw list is dropped, so the Rc is normally unique.
        if let Some(e) = self.text_entry.as_mut().and_then(Rc::get_mut) {
            e.x = r.x;
            e.y = r.y;
        } else {
            self.text_entry = Some(self.build_entry(ctx, r.x, r.y));
        }
    }

    fn paint(&self, ctx: &mut RenderContext) {
        if let Some(text_entry) = &self.text_entry {
            ctx.text(text_entry.clone());
        }
    }
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            id: ViewId::new(),
            text: text.into(),
            text_entry: None,
        }
    }
}

impl IntoView for String {
    type V = Label;
    type Intermediate = Label;

    fn into_intermediate(self) -> Label {
        Label::new(self)
    }
}

impl IntoView for &str {
    type V = Label;
    type Intermediate = Label;

    fn into_intermediate(self) -> Label {
        Label::new(self)
    }
}
