use crate::ui::render::QuadInstance;

use super::{UiNode, UiRenderKind, UiRenderQueue};

pub struct Image {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub texture_index: u32,

	pub children: Vec<Box<dyn UiNode>>
}

impl UiNode for Image {
	fn render_kind(&self) -> UiRenderKind {
		UiRenderKind::Quad
	}

	fn write_render_data(&self, out: &mut UiRenderQueue) {
		out.quads.push(self.into());
	}
	
	fn children(&self) -> &[Box<dyn UiNode>] {
		&self.children
	}
	
	fn children_mut(&mut self) -> &mut Vec<Box<dyn UiNode>> {
		&mut self.children
	}
}

impl From<&Image> for QuadInstance {
	fn from(value: &Image) -> Self {
		QuadInstance {
			color: value.color,
			position: value.position,
			size: value.size,
			texture_index: value.texture_index,
    		_padding: [0,0,0],
		}
	}
}