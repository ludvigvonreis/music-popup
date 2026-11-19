use crate::view::{View, view_id::ViewId};

pub struct Empty {
	pub(crate) id: ViewId,
}

impl Empty {
	pub fn new() -> Self {
		Empty { id: ViewId::new() }
	}
}

impl Default for Empty {
	fn default() -> Self {
		Self::new()
	}
}

impl View for Empty {
	fn id(&self) -> ViewId {
		self.id
	}
}