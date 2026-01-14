use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    protocol::{
        wl_callback, wl_compositor::WlCompositor, wl_display::WlDisplay, wl_registry::WlRegistry,
        wl_surface::WlSurface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, ZwlrLayerSurfaceV1},
};

use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use std::{fmt::Error, ptr::NonNull};
use wgpu::{RenderPass, SurfaceTargetUnsafe};

mod backend;
mod ui;
use crate::{
    backend::state::WGPUState,
    ui::{RenderUi, image::Image},
};

struct State {
    compositor: Option<WlCompositor>,
    layer_shell: Option<ZwlrLayerShellV1>,
    surface: Option<WlSurface>,
    layer_surface: Option<ZwlrLayerSurfaceV1>,
    configured: bool,
    redraw: bool,
}

impl State {
    fn new() -> Self {
        Self {
            compositor: None,
            layer_shell: None,
            surface: None,
            layer_surface: None,
            configured: false,
            redraw: false,
        }
    }
}

/* ---------- Registry handling ---------- */

impl Dispatch<WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            //println!("{}", interface.as_str());
            match interface.as_str() {
                "wl_compositor" => {
                    let compositor =
                        registry.bind::<WlCompositor, _, _>(name, version.min(4), qh, ());
                    println!("hello compositor");
                    state.compositor = Some(compositor);
                }
                "zwlr_layer_shell_v1" => {
                    let shell =
                        registry.bind::<ZwlrLayerShellV1, _, _>(name, version.min(4), qh, ());
                    println!("hello shell");
                    state.layer_shell = Some(shell);
                }
                _ => {}
            }
        }
    }
}

/* ---------- Layer surface events ---------- */

impl Dispatch<ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        surface: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                surface.ack_configure(serial);
                state.configured = true;
				state.redraw = true;

                // Later: resize wgpu surface here
                let _ = (width, height);
            }
            wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event::Closed => {
                std::process::exit(0);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for State {
    fn event(
        state: &mut Self,
        _callback: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.redraw = true;
        }
    }
}

/* ---------- Unused but required ---------- */

impl Dispatch<WlCompositor, ()> for State {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: <WlCompositor as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: <ZwlrLayerShellV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSurface, ()> for State {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: <WlSurface as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

/* ---------- main ---------- */

fn main() {
    pollster::block_on(run());
}

async fn run() {
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&qh, ());
    let mut state = State::new();

    // Roundtrip to gather globals
    event_queue.roundtrip(&mut state).unwrap();

    let compositor = state.compositor.as_ref().unwrap();
    let layer_shell = state.layer_shell.as_ref().unwrap();

    let surface = compositor.create_surface(&qh, ());
    state.surface = Some(surface.clone());

    let layer_surface = layer_shell.get_layer_surface(
        &surface,
        None,
        Layer::Overlay,
        "music-popup".into(),
        &qh,
        (),
    );

    layer_surface.set_anchor(Anchor::Top | Anchor::Right);
    layer_surface.set_margin(40, 40, 0, 0);
    layer_surface.set_exclusive_zone(-1);
    layer_surface.set_size(500, 500);

    surface.commit();

    state.layer_surface = Some(layer_surface);

    let mut wgpu_state = WGPUState::new(display, surface).await;
	
	let img2 = Image {
		color: [0.0, 1.0, 0.0, 1.0], // Red
		children: Vec::new(),
		position: [0.0, 0.5],
		size: [1.0, 2.0],
		texture_index: 0,
	};
    let img = Image {
        color: [1.0, 0.0, 1.0, 1.0], // Red
        children: vec![/*Box::new(img2)*/],
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        texture_index: 0,
    };


    let mut ui_renderer = RenderUi::new(Box::new(img));
    ui_renderer.create_buffers(&wgpu_state);
	ui_renderer.create_pipelines(&wgpu_state);

    // Event loop
    loop {
        event_queue.blocking_dispatch(&mut state).unwrap();
        if state.configured && state.redraw {
            render(&mut wgpu_state, |state, pass| {
				ui_renderer.render(&state, pass);
			});

            let surface = state.surface.as_ref().unwrap();
            surface.frame(&qh, ());
            surface.commit();

            state.redraw = false;
        }
    }
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

    let color: wgpu::Color = wgpu::Color::WHITE;

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
