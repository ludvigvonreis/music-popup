use crate::{
    types::Vec2,
    view::{HasViewId, IntoView},
};
use wgpu::Color;

#[derive(Clone, Copy)]
pub struct Style {
    pub background_color: Color,
    pub position: Vec2,
    pub size: Vec2,
    pub border_radius: f32,
}

impl Style {
    pub fn new() -> Self {
        Style {
            background_color: Color::WHITE,
            position: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 0.0, y: 0.0 },
            border_radius: 0.0,
        }
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    pub fn position(mut self, pos: Vec2) -> Self {
        self.position = pos;
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
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
