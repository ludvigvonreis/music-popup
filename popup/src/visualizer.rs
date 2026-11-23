use std::any::Any;

use library::{reactive::create_effect, view::update::push_update, view_prelude::*};

pub fn visualizer(bars: usize, f: impl Fn() -> Vec<u32> + 'static) -> Visualizer {
    let v = Visualizer::new(bars);
    let id = v.id;
    create_effect(move || {
        let s: Vec<u32> = f();
        push_update(id, Box::new(s));
    });
    v
}

pub struct Visualizer {
    id: ViewId,
    bars: usize,
    levels: Vec<u32>,
}

impl Visualizer {
    pub fn new(bars: usize) -> Self {
        let id = ViewId::new();
        Visualizer {
            id,
            bars,
            levels: Default::default(),
        }
    }
}

impl View for Visualizer {
    fn id(&self) -> ViewId {
        self.id
    }

    fn update(&mut self, state: Box<dyn Any>) {
        if let Ok(levels) = state.downcast::<Vec<u32>>() {
            self.levels = *levels;
        }
    }

    fn paint(&self, ctx: &mut RenderContext) {
        let s = self.id.state().borrow().style.clone();
        let r = self.id.rect();

        let rgba = |c: &Color| [c.r as f32, c.g as f32, c.b as f32, c.a as f32];

        let bars = self.bars;
        let gap = 2.0;
        let bar_w = (r.w - gap * (bars as f32 - 1.0)) / bars as f32;

        for i in 0..bars {
            let level =
                self.levels.get(i).copied().unwrap_or(0) as f32 / crate::audio::LEVEL_MAX as f32;
            let h = r.h * level.clamp(0.0, 1.0);

            ctx.rect(
                RectInstance {
                    position: [r.x + (bar_w + gap) * i as f32, r.y + r.h - h],
                    size: [bar_w, h],
                    color: rgba(&s.background_color),
                    border_color: [0.0; 4],
                    shadow_color: [0.0; 4],
                    shadow_offset: [0.0; 2],
                    radius: s.border_radius,
                    border_width: 0.0,
                    shadow_blur: 0.0,
                    _padding: [0.0; 3],
                },
                None,
            );
        }
    }
}
