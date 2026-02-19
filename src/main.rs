use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Anchor;
use wgpu::{Color, RenderPass};

mod backend;
mod style;
mod types;
mod ui;
mod view;
mod views;

use crate::{
    backend::{render::Renderer, render_context::RenderContext, state::WGPUState, window::Window},
    style::Decorators,
    types::{ObjectData, Vec2},
    ui::{RenderUi, image::Image},
    view::{IntoView, View},
    views::{div, label, text},
};

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

    let mut renderer = Renderer::new(wgpu_state);

    // let img = Image {
    //     color: [0.368, 0.611, 0.145, 1.0], // Red
    //     children: vec![/* Box::new(img2)*/],
    //     position: [0.0, 0.0],
    //     size: [340.0, 90.0],
    //     texture_index: 0,
    // };

    // let mut ui_renderer = RenderUi::new(Box::new(img));

    // ui_renderer.create_buffers(&wgpu_state);
    // ui_renderer.create_bind_groups(&wgpu_state);
    // ui_renderer.create_pipelines(&wgpu_state);

    window.event_loop(|| {
        renderer.render_frame(app_view());
        // render(&mut wgpu_state, |state, pass| {
        //     ui_renderer.render(&state, pass);
        // });
    });
}

fn app_view() -> impl IntoView {
    let label1 = label(move || "Hello World");
    let label2 = label(move || "My name is");
    let label3 = label(move || "What?");
    let label4 = label(move || "Who?");

    div((div((label1,)).style(|s| {
        s.background(Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        })
        .border_radius(5.0)
        .position(Vec2 { x: 85.0, y: 0.0 })
        .size(Vec2 { x: 170.0, y: 90.0 })
    }),))
    .style(|s| {
        s.background(Color {
            r: 0.368,
            g: 0.611,
            b: 0.145,
            a: 1.0,
        })
        .border_radius(10.0)
        .position(Vec2 { x: -85.0, y: 0.0 })
        .size(Vec2 { x: 170.0, y: 90.0 })
    })
}
