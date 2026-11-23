use crate::{
    color::Color,
    reactive::create_effect,
    vector::Vec2,
    view::update::{SetStyle, push_update},
    view::{HasViewId, IntoView},
};
use std::{cell::RefCell, rc::Rc};
use taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, Rect, Size,
    style_helpers::{length, percent},
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
    pub layout: taffy::Style,
    pub border_color: Color,
    pub border_width: f32,
    pub shadow: Option<Shadow>,
    pub clip_children: bool,
}

#[derive(Clone)]
pub struct Shadow {
    pub offset: (f32, f32),
    pub blur: f32,
    pub color: Color,
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
            layout: taffy::Style::default(),
            border_color: Color::from_hex("#00000000"),
            border_width: 0.0,
            shadow: None,
            clip_children: false,
        }
    }
}

impl Style {
    pub fn clip(mut self) -> Self {
        self.clip_children = true;
        self.layout.overflow = taffy::Point {
            x: taffy::Overflow::Hidden,
            y: taffy::Overflow::Hidden,
        };
        self
    }

    /// Inset border. Also shrinks the content box, like CSS border-box.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self.layout.border = Rect {
            left: length(width),
            right: length(width),
            top: length(width),
            bottom: length(width),
        };
        self
    }

    pub fn box_shadow(mut self, x: f32, y: f32, blur: f32, color: Color) -> Self {
        self.shadow = Some(Shadow {
            offset: (x, y),
            blur,
            color,
        });
        self
    }
    pub fn width(mut self, v: Dimension) -> Self {
        self.layout.size.width = v;
        self
    }
    pub fn height(mut self, v: Dimension) -> Self {
        self.layout.size.height = v;
        self
    }
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.layout.size = Size {
            width: length(w),
            height: length(h),
        };
        self
    }
    pub fn min_width(mut self, v: Dimension) -> Self {
        self.layout.min_size.width = v;
        self
    }
    pub fn flex_row(mut self) -> Self {
        self.layout.flex_direction = FlexDirection::Row;
        self
    }
    pub fn flex_column(mut self) -> Self {
        self.layout.flex_direction = FlexDirection::Column;
        self
    }
    pub fn justify(mut self, v: JustifyContent) -> Self {
        self.layout.justify_content = Some(v);
        self
    }
    pub fn align(mut self, v: AlignItems) -> Self {
        self.layout.align_items = Some(v);
        self
    }
    pub fn gap(mut self, v: f32) -> Self {
        self.layout.gap = Size {
            width: length(v),
            height: length(v),
        };
        self
    }
    pub fn padding(mut self, v: f32) -> Self {
        self.layout.padding = Rect {
            left: length(v),
            right: length(v),
            top: length(v),
            bottom: length(v),
        };
        self
    }
    pub fn margin(mut self, v: f32) -> Self {
        self.layout.margin = Rect {
            left: length(v),
            right: length(v),
            top: length(v),
            bottom: length(v),
        };
        self
    }
    pub fn grow(mut self, v: f32) -> Self {
        self.layout.flex_grow = v;
        self
    }
    pub fn shrink(mut self, v: f32) -> Self {
        self.layout.flex_shrink = v;
        self
    }
    pub fn basis(mut self, v: Dimension) -> Self {
        self.layout.flex_basis = v;
        self
    }

    pub fn absolute_fill(mut self) -> Self {
        self.layout.position = taffy::Position::Absolute;
        self.layout.inset = Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(0.0_f32),
        };
        self
    }

    pub fn absolute_inset(mut self, v: f32) -> Self {
        self.layout.position = taffy::Position::Absolute;
        self.layout.inset = Rect {
            left: length(v),
            right: length(v),
            top: length(v),
            bottom: length(v),
        };
        self
    }
}

impl_style_setters! {
    Style {
        background_color: Color,
        border_radius: f32,
        font_size: f32,
        line_height: f32,
        wrap: glyphon::Wrap,
        weight: glyphon::Weight,
        text_style: glyphon::Style,
        text_color: Color,
        font_family: String,
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

pub fn px(v: f32) -> Dimension {
    length(v)
}
pub fn pct(v: f32) -> Dimension {
    percent(v)
}

impl<T: IntoView> Decorators for T {}
