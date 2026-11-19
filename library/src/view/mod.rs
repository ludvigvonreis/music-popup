pub mod view_id;
use std::any::Any;

use view_id::*;

use crate::backend::{layout_context::LayoutContext, render_context::RenderContext};
pub mod state;
pub mod storage;
pub mod update;
pub mod view_iter;

pub type AnyView = Box<dyn View>;

pub trait IntoView: Sized {
    type V: View + 'static;
    type Intermediate: HasViewId + IntoView<V = Self::V>;

    fn into_intermediate(self) -> Self::Intermediate;

    fn into_view(self) -> Self::V {
        self.into_intermediate().into_view()
    }

    fn into_any(self) -> AnyView {
        Box::new(self.into_view())
    }
}

pub trait HasViewId {
    fn view_id(&self) -> ViewId;
}

impl<V: View> HasViewId for V {
    fn view_id(&self) -> ViewId {
        self.id()
    }
}

impl<VW: View + 'static> IntoView for VW {
    type V = VW;
    type Intermediate = Self;

    fn into_intermediate(self) -> Self::Intermediate {
        self
    }

    fn into_view(self) -> Self::V {
        self
    }
}

pub trait View {
    fn id(&self) -> ViewId;

    fn debug_name(&self) -> std::borrow::Cow<'static, str> {
        core::any::type_name::<Self>().into()
    }

    fn layout(&mut self, ctx: &mut LayoutContext) {
        self.id()
            .children()
            .iter()
            .for_each(|f| f.view().borrow_mut().layout(ctx));
    }

    fn paint(&self, ctx: &mut RenderContext) {
        self.id()
            .children()
            .iter()
            .for_each(|f| f.view().borrow().paint(ctx));
    }

    fn update(&mut self, state: Box<dyn Any>) {}
}
