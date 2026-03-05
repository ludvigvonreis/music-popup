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

    transparent_rect(div(label1).style(|s| {
        s.background(Color::from_hex("#000000ff"))
            .border_radius(10.0)
            .position(Vec2::new(-10.0, 0.0))
            .size(Vec2::new(300.0, 50.0))
    }))
}

fn transparent_rect(children: impl IntoViewIter) -> impl IntoView {
    div(children).style(|s| {
        s.background(Color::from_hex("#21273344"))
            .border_radius(10.0)
            .position(Vec2::new(0.0, 0.0))
            .size(Vec2::new(340.0, 90.0))
    })
}
