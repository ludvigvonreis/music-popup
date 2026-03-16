#![allow(dead_code)]
mod backend;
mod color;
mod style;
mod types;
mod vector;
mod view;
mod views;

use crate::{
    backend::{render::Renderer, state::WGPUState, window::Window},
    color::Color,
    style::Decorators,
    vector::Vec2,
    view::{IntoView, view_iter::IntoViewIter},
    views::{div, text},
};

use glyphon::Weight;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Anchor;

fn main() {
    pollster::block_on(run());
}

async fn run() {
    let mut window = Window::new(
        "music-popup",
        (380, 80),
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

    let mut renderer = Renderer::new(wgpu_state, music_status());

    window.event_loop(|| {
        renderer.layout_frame();
        renderer.render_frame();
    });
}
fn track_info() -> impl IntoView {
    let title = text("Virtual Insanity").style(|s| {
        s.text_color(Color::from_hex("#d9d7ce")) // foreground
            .font_size(17.0)
            .weight(Weight::SEMIBOLD)
            .position(Vec2::new(62.0, 10.0))
    });
    let artist = text("Jamiroquai").style(|s| {
        s.text_color(Color::from_hex("#b5b3ac")) // color8 (muted)
            .font_size(13.0)
            .weight(Weight::NORMAL)
            .position(Vec2::new(62.0, 30.0))
    });
    div((title, artist)).style(|s| s.size(Vec2::new(360.0, 36.0)))
}

fn album_art() -> impl IntoView {
    div(()).style(|s| {
        s.background_color(Color::from_hex("#343f4c")) // selection_background
            .border_radius(4.0)
            .position(Vec2::new(8.0, 8.0))
            .size(Vec2::new(44.0, 44.0))
    })
}

fn progress_bar() -> impl IntoView {
    div((
        div(()).style(|s| {
            s.background_color(Color::from_hex("#343f4c")) // selection_background (track)
                .border_radius(4.0)
                .position(Vec2::new(8.0, 60.0))
                .size(Vec2::new(364.0, 5.0))
        }),
        div(()).style(|s| {
            s.background_color(Color::from_hex("#cfbafa")) // cursor/accent (fill)
                .border_radius(4.0)
                .position(Vec2::new(8.0, 60.0))
                .size(Vec2::new(184.0, 5.0))
        }),
    ))
}

fn transparent_rect(children: impl IntoViewIter) -> impl IntoView {
    div(children).style(|s| {
        s.border_radius(10.0)
            .position(Vec2::new(0.0, 0.0))
            .size(Vec2::new(380.0, 80.0))
            .background_color(Color::from_hex("#212733cc"))
    })
}

fn music_status() -> impl IntoView {
    transparent_rect((album_art(), track_info(), progress_bar()))
}
