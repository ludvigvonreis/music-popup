use std::{
    io::{Read, Write},
    os::{fd::AsRawFd, unix::net::UnixStream},
    sync::Arc,
    time::Duration,
};
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle,
    protocol::{
        wl_callback, wl_compositor::WlCompositor, wl_display::WlDisplay, wl_registry::WlRegistry,
        wl_surface::WlSurface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, ZwlrLayerSurfaceV1},
};

#[derive(Clone)]
pub struct Waker {
    tx: Arc<UnixStream>,
}

pub struct WakeRx {
    rx: UnixStream,
}

pub fn wake_channel() -> std::io::Result<(Waker, WakeRx)> {
    let (tx, rx) = UnixStream::pair()?;
    tx.set_nonblocking(true)?;
    rx.set_nonblocking(true)?;
    Ok((Waker { tx: Arc::new(tx) }, WakeRx { rx }))
}

impl Waker {
    /// Safe to call from any thread.
    pub fn wake(&self) {
        let _ = (&*self.tx).write(&[1]);
    }
}

impl WakeRx {
    fn drain(&self) {
        let mut buf = [0u8; 64];
        while let Ok(n) = (&self.rx).read(&mut buf) {
            if n == 0 {
                break;
            }
        }
    }
}

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

    pub fn event_loop<T, D>(&mut self, wake: &WakeRx, mut tick: T, mut draw: D)
    where
        T: FnMut() -> Option<Duration>,
        D: FnMut() -> bool,
    {
        let mut timeout: Option<Duration> = None;
        loop {
            self.wait_and_dispatch(wake, timeout);
            wake.drain();
            timeout = tick();

            // `redraw` is set by the compositor's frame callback / configure.
            // While idle it stays true, so a signal set by `tick` or by an
            // input event draws immediately.
            if self.configured && self.redraw {
                if draw() {
                    let qh = self.qh.as_ref().unwrap();
                    let surface = self.surface.as_ref().unwrap();
                    surface.frame(qh, ());
                    surface.commit();
                    self.redraw = false;
                }
            }
        }
    }

    fn wait_and_dispatch(&mut self, wake: &WakeRx, timeout: Option<Duration>) {
        let mut queue = self.event_queue.take().expect("event_queue missing");

        queue.dispatch_pending(self).unwrap();
        let _ = queue.flush();

        if let Some(guard) = queue.prepare_read() {
            let wl_fd = guard.connection_fd().as_raw_fd();
            let mut fds = [
                libc::pollfd {
                    fd: wl_fd,
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: wake.rx.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            let ms = timeout.map_or(-1, |d| d.as_millis().min(i32::MAX as u128) as i32);
            // EINTR and timeouts are fine: we just fall through and loop.
            unsafe { libc::poll(fds.as_mut_ptr(), 2, ms) };

            if fds[0].revents & libc::POLLIN != 0 {
                let _ = guard.read();
            } else {
                drop(guard); // cancel the read; nothing from the compositor
            }
            queue.dispatch_pending(self).unwrap();
        }

        self.event_queue = Some(queue);
    }
}
