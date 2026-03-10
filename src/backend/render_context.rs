use glyphon::TextArea;

use crate::{
    backend::render_storage::{PipelineType, RenderStorage},
    types::{RectInstance, TextEntry, Vertex},
};
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::Range,
    rc::Rc,
};

pub struct RenderContext {
    draw_list: DrawList,
    render_storage: Rc<RefCell<RenderStorage>>,
}

//#[derive(Debug)]
pub struct DrawList {
    pub commands: Vec<DrawCommand>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub rect_instances: Vec<RectInstance>,
    pub text_instances: Vec<Rc<TextEntry>>,
}

#[derive(Debug, Clone)]
pub struct DrawCommand {
    pub kind: DrawKind,
    pub pipeline: PipelineType,
    pub clip: Option<ClipRect>, // scissor rect; None = no clipping
}

#[derive(Debug, Clone)]
pub enum DrawKind {
    Rects { instance_range: Range<u32> },
    Glyphs,
    // Mesh {
    //     vertex_range: Range<u32>,
    //     index_range: Range<u32>,
    //     texture_id: u32, // for image quads
    // },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Default for DrawList {
    fn default() -> Self {
        Self {
            commands: Default::default(),
            vertices: Default::default(),
            indices: Default::default(),
            rect_instances: Default::default(),
            text_instances: Default::default(),
        }
    }
}

impl RenderContext {
    pub fn new(render_storage: Rc<RefCell<RenderStorage>>) -> RenderContext {
        RenderContext {
            draw_list: Default::default(),
            render_storage: render_storage,
        }
    }

    pub fn draw_list(&self) -> &DrawList {
        &self.draw_list
    }

    pub fn storage(&self) -> Ref<RenderStorage> {
        self.render_storage.borrow()
    }

    pub fn storage_mut(&self) -> RefMut<RenderStorage> {
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

        self.draw_list.commands.push(DrawCommand {
            kind: DrawKind::Glyphs,
            pipeline: PipelineType::Text,
            clip: None,
        });
    }
}
