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
use wgpu::SurfaceTargetUnsafe;

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

struct WGPUState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    surface_format: wgpu::TextureFormat,

    counter: i32,
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
    layer_surface.set_size(300, 80);

    surface.commit();

    state.layer_surface = Some(layer_surface);

    let mut wgpu_state = create_pipeline(display, surface).await.unwrap();

    // Event loop
    loop {
        event_queue.blocking_dispatch(&mut state).unwrap();
        if state.configured && state.redraw {
            render(&mut wgpu_state);

            let surface = state.surface.as_ref().unwrap();
            surface.frame(&qh, ());
            surface.commit();

            state.redraw = false;
        }
    }
}

fn render(state: &mut WGPUState) {
    state.counter += 1;

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

    let color: wgpu::Color;
	let color = if (state.counter / 100) % 2 == 0 {
		wgpu::Color::RED
	} else {
		wgpu::Color::GREEN
	};


    // Renders a GREEN screen
    let mut encoder = state.device.create_command_encoder(&Default::default());
    // Create the renderpass which will clear the screen.
    let renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

    // End the renderpass.
    drop(renderpass);

    // Submit the command in the queue to execute
    state.queue.submit([encoder.finish()]);
    surface_texture.present();
}

async fn create_pipeline(display: WlDisplay, surface: WlSurface) -> Result<WGPUState, Error> {
    let handle_window: RawWindowHandle = {
        let surface_id = surface.id();
        let pointer = NonNull::new(surface_id.as_ptr() as *mut _).expect("window pointer");
        RawWindowHandle::Wayland(WaylandWindowHandle::new(pointer))
    };
    let handle_display: RawDisplayHandle = {
        let display_id = display.id();
        let pointer = NonNull::new(display_id.as_ptr() as *mut _).expect("display pointer");
        RawDisplayHandle::Wayland(WaylandDisplayHandle::new(pointer))
    };

    let surface_target = SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: handle_display.clone(),
        raw_window_handle: handle_window.clone(),
    };

    let instance = wgpu::Instance::default();
    let surface = unsafe {
        instance
            .create_surface_unsafe(surface_target)
            .expect("surface unsafe")
    };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Adapter");

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        })
        .await
        .expect("Device");

    let surface_caps = surface.get_capabilities(&adapter);
    // Shader code in this tutorial assumes an sRGB surface texture. Using a different
    // one will result in all the colors coming out darker. If you want to support non
    // sRGB surfaces, you'll need to account for that when drawing to the frame.
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: 300,
        height: 80,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    surface.configure(&device, &config);

    let cap = surface.get_capabilities(&adapter);
    let surface_format = cap.formats[0];

    Ok(WGPUState {
        surface,
        device,
        queue,
        config,
        surface_format,
        counter: 0,
    })
}
