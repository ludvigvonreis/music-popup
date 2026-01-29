use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Anchor;
use wgpu::{Color, RenderPass};

mod backend;
mod style;
mod ui;
mod view;
mod views;
use crate::{
    backend::{state::WGPUState, window::Window},
    style::Decorators,
    ui::{RenderUi, image::Image},
    view::{IntoView, View},
    views::{div, label, text},
};

fn main() {
    pollster::block_on(run());
}

async fn run() {
    app_view().into_view().print();

    let mut window = Window::new(
        "Music Widget",
        (350, 100),
        (540 - (150), 960 - (350 / 2), 0, 0),
        Anchor::Top | Anchor::Right,
    );

    let mut wgpu_state = {
        let (display, surface) = match (&window.display, &window.surface) {
            (Some(d), Some(s)) => (d, s),
            _ => return,
        };

        WGPUState::new(&display, &surface, window.size()).await
    };

    let img = Image {
        color: [0.368, 0.611, 0.145, 1.0], // Red
        children: vec![/*Box::new(img2)*/],
        position: [0.0, 0.0],
        size: [340.0, 90.0],
        texture_index: 0,
    };

    let mut ui_renderer = RenderUi::new(Box::new(img));

    ui_renderer.create_buffers(&wgpu_state);
    ui_renderer.create_bind_groups(&wgpu_state);
    ui_renderer.create_pipelines(&wgpu_state);

    window.event_loop(|| {
        render(&mut wgpu_state, |state, pass| {
            ui_renderer.render(&state, pass);
        });
    });
}

fn app_view() -> impl IntoView {
    let label1 = label(move || "Hello World");
    let label2 = label(move || "My name is");
    let label3 = label(move || "What?");
    let label4 = label(move || "Who?");

    div((
		label1.style(|mut s| s.background(Color::GREEN)), 
		label2,
		div((label3, label4)),
		div((text("hello"), text("Chicka chicka")))
	))
}

fn render<F>(state: &mut WGPUState, mut f: F)
where
    F: FnMut(&mut WGPUState, &mut RenderPass),
{
    // Create texture view
    let surface_texture = state
        .surface
        .get_current_texture()
        .expect("failed to acquire next swapchain texture");
    let texture_view = surface_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor {
            // Without add_srgb_suffix() the image we will be working with
            // might not be "gamma correct".
            format: Some(state.surface_format.add_srgb_suffix()),
            ..Default::default()
        });

    let color: wgpu::Color = wgpu::Color::TRANSPARENT;

    // Renders a GREEN screen
    let mut encoder = state.device.create_command_encoder(&Default::default());
    // Create the renderpass which will clear the screen.
    let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &texture_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    // If you wanted to call any drawing commands, they would go here.

    f(state, &mut renderpass);

    // End the renderpass.
    drop(renderpass);

    // Submit the command in the queue to execute
    state.queue.submit([encoder.finish()]);
    surface_texture.present();
}
