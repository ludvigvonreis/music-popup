use crate::{backend::render_storage::PipelineType, color::Color, types::Vertex, vector::Vec2};
use std::ops::Range;

pub struct RenderContext {
    draw_list: DrawList,
}

#[derive(Debug)]
pub struct DrawList {
    pub commands: Vec<DrawCommand>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub instanced_data: Vec<Instance>,
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub pos: Vec2,
    pub size: Vec2,
    pub color: Color,
    pub radius: f32,
}

#[derive(Debug, Clone)]
pub struct DrawCommand {
    pub pipeline: PipelineType,
    pub vertex_range: Range<u64>,
    pub index_range: Range<u64>,
    //pub instance_range: Range<u64>,
    pub texture_index: u32,
}

impl Default for RenderContext {
    fn default() -> Self {
        Self {
            draw_list: Default::default(),
        }
    }
}

impl Default for DrawList {
    fn default() -> Self {
        Self {
            commands: Default::default(),
            vertices: Default::default(),
            indices: Default::default(),
            instanced_data: Default::default(),
        }
    }
}

impl RenderContext {
    pub fn draw_list(&self) -> &DrawList {
        &self.draw_list
    }

    pub fn rect(&mut self, pos: Vec2, size: Vec2, radius: f32, color: Color) {
        const VERTICES: &[Vertex] = &[
            Vertex {
                position: [-1.0, -1.0],
            },
            Vertex {
                position: [1.0, -1.0],
            },
            Vertex {
                position: [1.0, 1.0],
            },
            Vertex {
                position: [-1.0, 1.0],
            },
        ];

        const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

        let base_vertices: u64 =
            u64::try_from(self.draw_list.vertices.len()).expect("length does not fit in u64");
        let base_indices: u64 =
            u64::try_from(self.draw_list.indices.len()).expect("length does not fit in u64");

        for vertex in VERTICES {
            self.draw_list.vertices.push(*vertex);
        }

        for index in INDICES {
            self.draw_list.indices.push(*index);
        }

        let vertices_len: u64 = u64::try_from(VERTICES.len()).expect("length does not fit in u64");
        let indices_len: u64 = u64::try_from(INDICES.len()).expect("length does not fit in u64");

        self.draw_list.commands.push(DrawCommand {
            pipeline: PipelineType::Rect,
            vertex_range: (base_vertices * 8..(base_vertices + vertices_len) * 8),
            index_range: (base_indices * 2..(base_indices + indices_len) * 2),
            //instance_range: (base_instances * 48)..((base_instances + 1) * 48),
            texture_index: 0,
        });

        self.draw_list.instanced_data.push(Instance {
            pos,
            size,
            color,
            radius,
        });
    }

    // TODO: Implement this really with glyphon.
    pub fn text(&mut self) {
        todo!()
    }
}
