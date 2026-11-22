mod backend; // internal: Wayland, wgpu, layout, render

pub mod app;
pub mod color;
pub mod reactive;
pub mod remote;
pub mod style;
pub mod types;
pub mod vector;
pub mod view;
pub mod views;

/// Window-level controls a component can use.
pub mod window {
    pub use crate::backend::window::bind_visible;
}

// Re-exported so user crates don't need glyphon / wayland-protocols-wlr.
pub use glyphon::Weight;
pub use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Anchor;

/// `use library::prelude::*;` is all a typical app needs.
pub mod prelude {
    pub use crate::{
        Anchor, Weight,
        app::App,
        color::Color,
        reactive::RwSignal,
        remote::{RemoteSender, remote_signal, spawn},
        style::Decorators,
        vector::Vec2,
        view::{IntoView, view_iter::IntoViewIter},
        views::{div, label},
        window,
    };
}
