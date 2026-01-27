use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    protocol::{
        wl_callback, wl_compositor::WlCompositor, wl_display::WlDisplay, wl_registry::WlRegistry,
        wl_surface::WlSurface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, ZwlrLayerSurfaceV1},
};

#[derive(Debug)]
pub struct Window {
    pub compositor: Option<WlCompositor>,
    pub layer_shell: Option<ZwlrLayerShellV1>,
    pub surface: Option<WlSurface>,
    pub layer_surface: Option<ZwlrLayerSurfaceV1>,
    pub display: Option<WlDisplay>,
    pub event_queue: Option<EventQueue<Self>>,
    pub qh: Option<QueueHandle<Self>>,

    pub configured: bool,
    pub redraw: bool,

    size: (u32, u32),
	scaling_ratio: f32,
    title: String,
}
/* ---------- Registry handling ---------- */

impl Dispatch<WlRegistry, ()> for Window {
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
                    state.compositor = Some(compositor);
                }
                "zwlr_layer_shell_v1" => {
                    let shell =
                        registry.bind::<ZwlrLayerShellV1, _, _>(name, version.min(4), qh, ());
                    state.layer_shell = Some(shell);
                }
                _ => {}
            }
        }
    }
}

/* ---------- Layer surface events ---------- */

impl Dispatch<ZwlrLayerSurfaceV1, ()> for Window {
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

impl Dispatch<wl_callback::WlCallback, ()> for Window {
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

impl Dispatch<WlCompositor, ()> for Window {
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

impl Dispatch<ZwlrLayerShellV1, ()> for Window {
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

impl Dispatch<WlSurface, ()> for Window {
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

impl Window {
    pub fn new(
        title: &str,
        size: (u32, u32),
        margins: (i32, i32, i32, i32),
        anchor: Anchor,
    ) -> Self {
        let mut state = Self {
            compositor: None,
            layer_shell: None,
            surface: None,
            layer_surface: None,
            display: None,
            event_queue: None,
            qh: None,
            configured: false,
            redraw: false,
            size,
			scaling_ratio: (size.0 / size.1) as f32,
            title: title.to_owned(),
        };

        let conn = Connection::connect_to_env().unwrap();
        let display = conn.display();
        state.display = Some(display.clone());

        let mut event_queue = conn.new_event_queue();
        let qh = event_queue.handle();

        let _registry = display.get_registry(&qh, ());

        // Roundtrip to gather globals
        event_queue.roundtrip(&mut state).unwrap();
        state.event_queue = Some(event_queue);

        let compositor = state.compositor.as_ref().unwrap();
        let layer_shell = state.layer_shell.as_ref().unwrap();

        let surface = compositor.create_surface(&qh, ());
        state.surface = Some(surface.clone());

        let layer_surface =
            layer_shell.get_layer_surface(&surface, None, Layer::Overlay, title.into(), &qh, ());

        layer_surface.set_anchor(anchor);
        layer_surface.set_margin(margins.0, margins.1, margins.2, margins.3);
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_size(size.0, size.1);

        surface.commit();

        state.qh = Some(qh);

        state
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn resize(&self, size: (u32, u32)) -> &Self {
		if let Some(layer_surface) = &self.layer_surface {
			layer_surface.set_size(size.0, size.1);

			self.commit_surface();
		}
        self
    }

	fn commit_surface(&self) -> &Self {
		if let Some(surface) = &self.surface {
			surface.commit();
		}

		self
	}

    pub fn event_loop<F>(&mut self, mut f: F)
    where
        F: FnMut(),
    {
        loop {
            let mut event_queue = self.event_queue.take().expect("event_queue missing");

            event_queue.blocking_dispatch(self).unwrap();
			self.event_queue = Some(event_queue);

            if self.configured && self.redraw {
                let qh = self.qh.as_ref().unwrap();
                let surface = self.surface.as_ref().unwrap();

				f();

                surface.frame(qh, ());
                surface.commit();

                self.redraw = false;
            }
        }
    }
}
