//! `App` owns everything platform-shaped: the layer-shell window, wgpu, the
//! event loop, wakeups from worker threads, dirty-gated drawing. User code
//! only supplies a root component.

use std::sync::Arc;

use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Anchor;

use crate::{
    backend::{
        render::Renderer,
        state::WGPUState,
        window::{Window, wake_channel},
    },
    reactive::{create_scope, with_scope},
    remote::{drain_remote, set_wake_hook},
    view::{
        IntoView,
        update::{apply_updates, is_dirty, take_dirty},
    },
};

pub struct App {
    name: String,
    // Adjust these integer types to whatever `Window::new` expects.
    size: (u32, u32),
    margin: (i32, i32, i32, i32),
    anchor: Anchor,
}

impl App {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            size: (300, 100),
            margin: (0, 0, 0, 0),
            anchor: Anchor::empty(),
        }
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    /// Passed straight through to `Window::new`, same order as before.
    pub fn margin(mut self, a: i32, b: i32, c: i32, d: i32) -> Self {
        self.margin = (a, b, c, d);
        self
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Blocks until the window closes. `root` runs once, on the main thread,
    /// after the runtime and wake hook exist, so it may create signals and
    /// call `remote_signal` / `remote_events` / `spawn`.
    pub fn run<V: IntoView>(self, root: impl FnOnce() -> V) {
        pollster::block_on(self.run_async(root));
    }

    async fn run_async<V: IntoView>(self, root: impl FnOnce() -> V) {
        let mut window = Window::new(&self.name, self.size, self.margin, self.anchor);

        let wgpu_state = {
            let (display, surface) = match (&window.display, &window.surface) {
                (Some(d), Some(s)) => (d, s),
                _ => return,
            };
            WGPUState::new(display, surface, window.size()).await
        };

        // Worker threads reach the event loop through this and nothing else.
        let (waker, wake_rx) = wake_channel().expect("failed to create wake channel");
        set_wake_hook(Arc::new(move || waker.wake()));

        // Everything the app creates hangs off one scope, so a future
        // "close window" path can dispose it and stop all effects and workers.
        let app_scope = create_scope();
        let root_view = with_scope(app_scope, root);

        let mut renderer = Renderer::new(wgpu_state, root_view);

        window.event_loop(
            &wake_rx,
            // tick: move queued worker data into signals (one batch -> one
            // frame), then sleep until the next event or wake.
            || {
                drain_remote();
                None
            },
            // draw: no GPU work unless something is dirty.
            || {
                if !is_dirty() {
                    return false;
                }
                apply_updates();
                take_dirty();
                renderer.layout_frame();
                renderer.render_frame();
                true
            },
        );
    }
}
