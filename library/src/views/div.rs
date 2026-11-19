use crate::{
    backend::render_context::ClipRect,
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

    fn paint(&self, ctx: &mut crate::backend::render_context::RenderContext) {
        let style = self.id.state().borrow().style.clone();

        let pos = style.position;
        let size = style.size;
        let radius = style.border_radius;
        let color = style.background_color;

        let instance = RectInstance {
            position: [pos.x, pos.y],
            size: [size.x, size.y],
            color: [
                color.r as f32,
                color.g as f32,
                color.b as f32,
                color.a as f32,
            ],
            radius: radius,
            _padding: [0.0; 3],
        };

        let clip = ClipRect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };

        ctx.rect(instance, Some(clip));

        self.id()
            .children()
            .iter()
            .for_each(|f| f.view().borrow().paint(ctx));
    }
}
