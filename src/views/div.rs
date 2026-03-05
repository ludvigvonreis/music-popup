use crate::view::{View, view_id::ViewId, view_iter::IntoViewIter};

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

        ctx.rect(
            style.position,
            style.size,
            style.border_radius,
            style.background_color,
        );

        self.id()
            .children()
            .iter()
            .for_each(|f| f.view().borrow().paint(ctx));
    }
}
