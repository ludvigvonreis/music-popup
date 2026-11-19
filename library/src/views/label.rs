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

    fn layout(&mut self, ctx: &mut LayoutContext) {
        if self.text_entry.is_none() {
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

            self.text_entry = Some(Rc::new(TextEntry {
                buffer,
                x: style.position.x,
                y: style.position.y,
                color: style.text_color.into(),
            }));
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
