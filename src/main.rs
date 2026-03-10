mod backend;
mod color;
mod style;
mod types;
mod ui;
mod vector;
mod view;
mod views;

use crate::{
    backend::{render::Renderer, state::WGPUState, window::Window},
    color::Color,
    style::Decorators,
    vector::Vec2,
    view::{IntoView, view_iter::IntoViewIter},
    views::{div, label},
};

use glyphon::Weight;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Anchor;

fn main() {
    pollster::block_on(run());
}

async fn run() {
    let mut window = Window::new(
        "Music Widget",
        (350, 100),
        (540 - (150), 960 - (350 / 2), 0, 0),
        Anchor::Top | Anchor::Right,
    );

    let wgpu_state = {
        let (display, surface) = match (&window.display, &window.surface) {
            (Some(d), Some(s)) => (d, s),
            _ => return,
        };

        WGPUState::new(&display, &surface, window.size()).await
    };

    let mut renderer = Renderer::new(wgpu_state, app_view());

    window.event_loop(|| {
        renderer.layout_frame();
        renderer.render_frame();
    });
}

fn app_view() -> impl IntoView {
    let label1 = label(move || "Hello World\nShalom").style(|s| s.position(Vec2::new(30.0, 30.0)));
    let label2 = label(move || "What is your name...")
        .style(|s| s.weight(Weight::MEDIUM).position(Vec2::new(130.0, 30.0)));

    transparent_rect(div((label1, label2)).style(|s| {
        s.background_color(Color::from_hex("#ff0000aa"))
            .border_radius(10.0)
            .position(Vec2::new(-10.0, 0.0))
            .size(Vec2::new(300.0, 50.0))
    }))
}

fn transparent_rect(children: impl IntoViewIter) -> impl IntoView {
    div(children).style(|s| {
        s.background_color(Color::from_hex("#21273344"))
            .border_radius(10.0)
            .position(Vec2::new(0.0, 0.0))
            .size(Vec2::new(340.0, 90.0))
    })
}
