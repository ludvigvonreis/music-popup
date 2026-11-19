use crate::ui::render::{UiRenderKind, UiRenderQueue};

pub trait UiNode {
	fn render_kind(&self) -> UiRenderKind;
	fn write_render_data(&self, out: &mut UiRenderQueue);

	fn children(&self) -> &[Box<dyn UiNode>];
    fn children_mut(&mut self) -> &mut Vec<Box<dyn UiNode>>;
}

pub struct UiBox {
	pub children: Vec<Box<dyn UiNode>>
}

impl UiNode for UiBox {
	fn render_kind(&self) -> UiRenderKind {
		UiRenderKind::Nothing
	}

	fn write_render_data(&self, out: &mut UiRenderQueue) {}
	
	fn children(&self) -> &[Box<dyn UiNode>] {
		&self.children
	}
	
	fn children_mut(&mut self) -> &mut Vec<Box<dyn UiNode>> {
		&mut self.children
	}
}

impl UiBox {
	fn new() -> Self {
		Self { children: Vec::new() }
	}

	fn with_children(children: Vec<Box<dyn UiNode>>) -> Self {
		Self { children: children }
	}
}