use crate::{
    backend::render_context::RenderContext,
    color::Color,
    types::RectInstance,
    view::{View, view_id::ViewId, view_iter::IntoViewIter},
};

pub fn div(children: impl IntoViewIter) -> Div {
    Div::new(children)
}

pub struct Div {
    id: ViewId,
}

impl Div {
    pub fn new(children: impl IntoViewIter) -> Self {
        let id = ViewId::new();
        id.set_children_iter(children.into_view_iter());
        Div { id }
    }
}

impl View for Div {
    fn id(&self) -> ViewId {
        self.id
    }

    fn paint(&self, ctx: &mut RenderContext) {
        let s = self.id.state().borrow().style.clone();
        let r = self.id.rect();

        let rgba = |c: &Color| [c.r as f32, c.g as f32, c.b as f32, c.a as f32];
        let (shadow_color, shadow_offset, shadow_blur) = match &s.shadow {
            Some(sh) => (rgba(&sh.color), [sh.offset.0, sh.offset.1], sh.blur),
            None => ([0.0; 4], [0.0; 2], 0.0),
        };

        ctx.rect(
            RectInstance {
                position: [r.x, r.y],
                size: [r.w, r.h],
                color: rgba(&s.background_color),
                border_color: rgba(&s.border_color),
                shadow_color,
                shadow_offset,
                radius: s.border_radius,
                border_width: s.border_width,
                shadow_blur,
                _padding: [0.0; 3],
            },
            None,
        );

        self.id()
            .children()
            .iter()
            .for_each(|f| f.view().borrow().paint(ctx));
    }
}
