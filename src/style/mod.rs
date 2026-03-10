use glyphon::Family;

use crate::{
    color::Color,
    vector::Vec2,
    view::{HasViewId, IntoView},
};

macro_rules! impl_style_setters {
    (
        $struct:ident {
            $( $field:ident : $type:ty ),* $(,)?
        }
    ) => {
        impl $struct {
            $(
                pub fn $field(mut self, value: $type) -> Self {
                    self.$field = value;
                    self
                }
            )*
        }
    };
}

#[derive(Clone)]
pub struct Style {
    pub background_color: Color,
    pub position: Vec2,
    pub size: Vec2,
    pub border_radius: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub wrap: glyphon::Wrap,
    pub weight: glyphon::Weight,
    pub text_style: glyphon::Style,
    pub text_color: Color,
    pub font_family: String,
}

impl Style {
    pub fn new() -> Self {
        Style {
            background_color: Color::WHITE,
            position: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 0.0, y: 0.0 },
            border_radius: 0.0,
            font_size: 14.0,
            line_height: 1.2,
            wrap: glyphon::Wrap::None,
            weight: glyphon::Weight::NORMAL,
            text_style: glyphon::Style::Normal,
            text_color: Color::WHITE,
            font_family: "FiraCode Nerd Font".to_owned(),
        }
    }
}

impl_style_setters! {
    Style {
        background_color: Color,
        position: Vec2,
        size: Vec2,
        border_radius: f32,
        font_size: f32,
        line_height: f32,
        wrap: glyphon::Wrap,
        weight: glyphon::Weight,
        text_style: glyphon::Style,
        text_color: Color,
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
