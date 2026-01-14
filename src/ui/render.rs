use wgpu::{Device, util::DeviceExt};

use crate::backend::state::WGPUState;

use super::UiNode;

pub struct UiRenderQueue {
    pub quads: Vec<QuadInstance>,
    //text: Vec<TextInstance>,
    //paths: Vec<PathInstance>,
}

impl UiRenderQueue {
    pub fn new() -> Self {
        UiRenderQueue { quads: Vec::new() }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub texture_index: u32,
    pub _padding: [u32; 3], // pad to 16-byte alignment
}

impl QuadInstance {
    pub fn print(&self) {
        println!(
            "QuadInstance {{ position: [{:.2}, {:.2}], size: [{:.2}, {:.2}], color: [{:.2}, {:.2}, {:.2}, {:.2}], texture_index: {}, _padding: [{}, {}, {}] }}",
            self.position[0], self.position[1],
            self.size[0], self.size[1],
            self.color[0], self.color[1], self.color[2], self.color[3],
            self.texture_index,
            self._padding[0], self._padding[1], self._padding[2]
        );
    }
}

struct QuadBuffers {
    vertex_buffer: wgpu::Buffer, // static quad geometry
    index_buffer: wgpu::Buffer,  // static quad indices

    instance_buffer: wgpu::Buffer, // per-widget instance data
    instance_capacity: usize,      // how many instances can the buffer hold
}

pub enum UiRenderKind {
    Quad,
    Nothing,
}

pub struct RenderUi {
    root: Box<dyn UiNode>,
    queue: Option<UiRenderQueue>,

    quad_pipeline: Option<wgpu::RenderPipeline>,
    quad_buffers: Option<QuadBuffers>,
    num_quads: u32,
}

// &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0]

impl RenderUi {
    pub fn new(root: Box<dyn UiNode>) -> Self {
        RenderUi {
            root: root,
            queue: None,
            quad_pipeline: None,
            quad_buffers: None,
            num_quads: 0,
        }
    }

    pub fn create_buffers(&mut self, state: &WGPUState) {
        const VERTICES: &[Vertex] = &[
			Vertex { position: [-0.25, -0.25] },
			Vertex { position: [0.25, -0.25] },
			Vertex { position: [0.25, 0.25] },
			Vertex { position: [-0.25, 0.25] },

			// Vertex { position: [0.0, 0.5] },
			// Vertex { position: [-0.5, -0.5] },
			// Vertex { position: [0.5, -0.5] },
		];

		const INDICES: &[u16] = &[
			0, 1, 2,
			2, 3, 0
		];

        let vertex_buffer = state.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = state.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_capacity = 100; // max widgets per frame
        let instance_buffer = state.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Instance Buffer"),
            size: (instance_capacity * std::mem::size_of::<QuadInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.quad_buffers = Some(QuadBuffers {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            instance_capacity,
        });
    }

    pub fn create_pipelines(&mut self, state: &WGPUState) {
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1, // position.xy -> vec2<f32>
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 2, // size.xy -> vec2<f32>
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 3, // color -> vec4<f32>
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 4, // texture_index -> u32
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        };

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x2, // vertex position -> @location(0)
            ],
        };

        let shader_source = include_str!("../../shaders/quad.wgsl");
        self.quad_pipeline = Some(state.create_pipeline(
            "Quad Pipeline",
            shader_source,
            &[vertex_buffer_layout, instance_layout],
        ));
    }

    pub fn update_instance_buffers(&mut self, state: &WGPUState) {
        if let Some(quad_buffers) = &self.quad_buffers {
            let queue = Self::collect_queue(&self.root);
            self.num_quads = queue.quads.len() as u32;

            state.write_buffer(
                &quad_buffers.instance_buffer,
                0,
                bytemuck::cast_slice(&queue.quads),
            );
        }
    }

    fn collect_queue(root: &Box<dyn UiNode>) -> UiRenderQueue {
        let mut queue = UiRenderQueue::new();

        Self::traverse_hierarchy(&mut queue, root);

        queue
    }

    fn traverse_hierarchy(queue: &mut UiRenderQueue, node: &Box<dyn UiNode>) {
        match node.render_kind() {
            UiRenderKind::Quad => {
                node.write_render_data(queue);
            }
            UiRenderKind::Nothing => (),
        }

        node.children()
            .iter()
            .for_each(|n| Self::traverse_hierarchy(queue, n));
    }

    pub fn render(&mut self, state: &WGPUState, pass: &mut wgpu::RenderPass) {
        self.update_instance_buffers(&state);

        if let (Some(quad_pipeline), Some(quad_buffers)) =
            (self.quad_pipeline.as_ref(), self.quad_buffers.as_ref())
        {
            pass.set_pipeline(&quad_pipeline);
            pass.set_vertex_buffer(0, quad_buffers.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, quad_buffers.instance_buffer.slice(..));
			
            pass.set_index_buffer(
                quad_buffers.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            pass.draw_indexed(0..6, 0, 0..self.num_quads as u32);
        }
    }
}
