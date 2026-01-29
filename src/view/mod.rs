pub mod view_id;
use view_id::*;
pub mod view_iter;
pub mod storage;
pub mod state;

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

	fn print(&self) {

	}
}
