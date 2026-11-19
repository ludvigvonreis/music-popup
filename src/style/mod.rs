use crate::{
    color::Color,
    reactive::create_effect,
    vector::Vec2,
    view::update::{SetStyle, push_update},
    view::{HasViewId, IntoView},
};
use std::{cell::RefCell, rc::Rc};

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
            background_color: Color::from_hex("#00000000"),
            position: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 0.0, y: 0.0 },
            border_radius: 0.0,
            font_size: 14.0,
            line_height: 1.2,
            wrap: glyphon::Wrap::None,
            weight: glyphon::Weight::NORMAL,
            text_style: glyphon::Style::Normal,
            text_color: Color::WHITE,
            font_family: "FiraCode Nerd".to_owned(),
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
    /// Adds a style layer. The closure runs inside an effect, so any signal it
    /// reads re-resolves this view's style (and only this view's).
    fn style(self, style: impl FnMut(Style) -> Style + 'static) -> Self::Intermediate {
        let intermediate = self.into_intermediate();
        let view_id = intermediate.view_id();

        view_id
            .state()
            .borrow_mut()
            .style_layers
            .push(Rc::new(RefCell::new(style)));

        create_effect(move || {
            // Clone the layer list out so no RefCell borrow is held while the
            // user closures run (they may read signals, or even other views).
            let layers = view_id.state().borrow().style_layers.clone();

            // Resolve from scratch so conditional styles don't stick.
            let mut resolved = Style::new();
            for layer in &layers {
                resolved = (layer.borrow_mut())(resolved);
            }
            push_update(view_id, Box::new(SetStyle(resolved)));
        });

        intermediate
    }
}

impl<T: IntoView> Decorators for T {}
