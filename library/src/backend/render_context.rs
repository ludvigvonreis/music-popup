use crate::{
    backend::render_storage::{PipelineType, RenderStorage},
    types::{ImageData, ImageId, ImageInstance, RectInstance, TextEntry, Vertex},
};
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::Range,
    rc::Rc,
};

pub struct RenderContext {
    draw_list: DrawList,
    render_storage: Rc<RefCell<RenderStorage>>,
    clip_stack: Vec<ClipRect>,
    glyphs_queued: bool,
}

//#[derive(Debug)]
pub struct DrawList {
    pub commands: Vec<DrawCommand>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub rect_instances: Vec<RectInstance>,
    pub text_instances: Vec<Rc<TextEntry>>,
    pub image_instances: Vec<ImageInstance>,
    pub image_assets: Vec<Rc<ImageData>>,
    pub text_clips: Vec<Option<ClipRect>>,
}

#[derive(Debug, Clone)]
pub struct DrawCommand {
    pub kind: DrawKind,
    pub pipeline: PipelineType,
    pub clip: Option<ClipRect>,
}

#[derive(Debug, Clone)]
pub enum DrawKind {
    Rects {
        instance_range: Range<u32>,
    },
    Glyphs,
    Images {
        image_id: ImageId,
        instance_range: Range<u32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ClipRect {
    /// Logical rect to whole pixels: round outward so edges never lose a pixel.
    pub fn from_rect(r: crate::view::layout::Rect) -> Self {
        let x0 = r.x.floor().max(0.0);
        let y0 = r.y.floor().max(0.0);
        let x1 = (r.x + r.w).ceil().max(x0);
        let y1 = (r.y + r.h).ceil().max(y0);
        ClipRect {
            x: x0 as u32,
            y: y0 as u32,
            width: (x1 - x0) as u32,
            height: (y1 - y0) as u32,
        }
    }

    pub fn intersect(&self, o: &ClipRect) -> ClipRect {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = (self.x + self.width).min(o.x + o.width).max(x0);
        let y1 = (self.y + self.height).min(o.y + o.height).max(y0);
        ClipRect {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }

    /// wgpu panics if the scissor leaves the render target.
    pub fn clamp_to(&self, w: u32, h: u32) -> ClipRect {
        let x = self.x.min(w);
        let y = self.y.min(h);
        ClipRect {
            x,
            y,
            width: self.width.min(w - x),
            height: self.height.min(h - y),
        }
    }
}

impl Default for DrawList {
    fn default() -> Self {
        Self {
            commands: Default::default(),
            vertices: Default::default(),
            indices: Default::default(),
            rect_instances: Default::default(),
            text_instances: Default::default(),
            image_instances: Default::default(),
            image_assets: Default::default(),
            text_clips: Default::default(),
        }
    }
}

impl RenderContext {
    pub fn new(render_storage: Rc<RefCell<RenderStorage>>) -> RenderContext {
        RenderContext {
            draw_list: Default::default(),
            render_storage: render_storage,
            clip_stack: Vec::new(),
            glyphs_queued: false,
        }
    }

    pub fn draw_list(&self) -> &DrawList {
        &self.draw_list
    }

    pub fn storage(&self) -> Ref<'_, RenderStorage> {
        self.render_storage.borrow()
    }

    pub fn storage_mut(&self) -> RefMut<'_, RenderStorage> {
        self.render_storage.borrow_mut()
    }

    pub fn rect(&mut self, instance: RectInstance, clip: Option<ClipRect>) {
        let start = self.draw_list.rect_instances.len() as u32;
        self.draw_list.rect_instances.push(instance);
        let end = self.draw_list.rect_instances.len() as u32;

        // Merge into previous command if same kind and same clip
        if let Some(cmd) = self.draw_list.commands.last_mut() {
            if let DrawKind::Rects { instance_range } = &mut cmd.kind {
                if cmd.clip == clip && instance_range.end == start {
                    instance_range.end = end;
                    return;
                }
            }
        }

        self.draw_list.commands.push(DrawCommand {
            kind: DrawKind::Rects {
                instance_range: start..end,
            },
            pipeline: PipelineType::Rect,
            clip: clip,
        });
    }

    pub fn text(&mut self, text_entry: Rc<TextEntry>) {
        self.draw_list.text_instances.push(text_entry);
        let clip = self.current_clip();
        self.draw_list.text_clips.push(clip);

        // glyphon renders every prepared area in one call, so queue it once.
        if !self.glyphs_queued {
            self.glyphs_queued = true;
            self.draw_list.commands.push(DrawCommand {
                kind: DrawKind::Glyphs,
                pipeline: PipelineType::Text,
                clip: None,
            });
        }
    }

    pub fn image(
        &mut self,
        image: &Rc<ImageData>,
        instance: ImageInstance,
        clip: Option<ClipRect>,
    ) {
        if !self.draw_list.image_assets.iter().any(|a| a.id == image.id) {
            self.draw_list.image_assets.push(image.clone());
        }

        let start = self.draw_list.image_instances.len() as u32;
        self.draw_list.image_instances.push(instance);
        let end = start + 1;

        if let Some(cmd) = self.draw_list.commands.last_mut() {
            if let DrawKind::Images {
                image_id,
                instance_range,
            } = &mut cmd.kind
            {
                if *image_id == image.id && cmd.clip == clip && instance_range.end == start {
                    instance_range.end = end;
                    return;
                }
            }
        }

        self.draw_list.commands.push(DrawCommand {
            kind: DrawKind::Images {
                image_id: image.id,
                instance_range: start..end,
            },
            pipeline: PipelineType::Image,
            clip,
        });
    }

    pub fn push_clip(&mut self, r: crate::view::layout::Rect) {
        let new = ClipRect::from_rect(r);
        let clip = match self.clip_stack.last() {
            Some(parent) => parent.intersect(&new),
            None => new,
        };
        self.clip_stack.push(clip);
    }

    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    fn current_clip(&self) -> Option<ClipRect> {
        self.clip_stack.last().copied()
    }
}
