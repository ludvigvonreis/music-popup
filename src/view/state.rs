use crate::{style::Style, view::{view_id::ViewId}};

pub struct ViewState {
	pub(crate) view_id: ViewId,
	pub(crate) style: Style,
}

impl ViewState {
	pub fn new(id: ViewId) -> Self {
		let style = Style::new();

		Self { view_id: id, style }
	}
}