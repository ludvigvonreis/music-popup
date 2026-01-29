use wgpu::Color;
use crate::view::{HasViewId, IntoView};

#[derive(Clone, Copy)]
pub struct Style {
	background_color: Color,
}

impl Style {
	pub fn new() -> Self {
		Style { background_color: Color::WHITE }
	}

	pub fn background(&mut self, color: Color) -> Self {
		self.background_color = color;

		*self
	}
}

pub trait Decorators: IntoView {
	fn style(self, mut style: impl FnMut(Style) -> Style + 'static) -> Self::Intermediate {
		let intermediate = self.into_intermediate();
		let view_id = intermediate.view_id();
		let state = view_id.state();

		// Update style to changed value
		let old_style = state.borrow_mut().style.clone();
		let new_style = style(old_style);
		state.borrow_mut().style = new_style;

		intermediate
	}
}

impl<T: IntoView> Decorators for T {}