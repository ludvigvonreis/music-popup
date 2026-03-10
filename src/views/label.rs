use std::rc::Rc;

use glyphon::{Attrs, Buffer, Family, Metrics};

use crate::{
    backend::{layout_context::LayoutContext, render_context::RenderContext},
    types::TextEntry,
    view::{View, view_id::ViewId},
};

pub fn label<S: Into<String>>(label: impl Fn() -> S + 'static) -> Label {
    Label::new(label())
}

// pub fn text<S: Display>(label: S) -> Label {
//     Label {
//         id: ViewId::new(),
//         value: label.to_string(),
//     }
// }

pub struct Label {
    id: ViewId,
    text: String,
    text_entry: Option<Rc<TextEntry>>,
}

impl View for Label {
    fn id(&self) -> ViewId {
        self.id
    }

    fn layout(&mut self, ctx: &mut LayoutContext) {
        if self.text_entry.is_none() {
            println!("layout text");
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
        let id = ViewId::new();
        let text = text.into();
        Self {
            id,
            text,
            text_entry: None,
        }
    }
}
