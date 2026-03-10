use std::collections::HashMap;

use glyphon::{
    Cache, FontSystem, Resolution, SwashCache, TextArea, TextAtlas, TextRenderer, Viewport,
};
use wgpu::{BindGroup, BindGroupLayout, Buffer, MultisampleState, RenderPipeline};

use crate::backend::state::WGPUState;
use crate::types::{RectInstance, Uniforms, Vertex};
use crate::view;

// Magic number.
// How many instances can be rendered at the same time.
const OBJECT_CAPACITY: usize = 1024;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum PipelineType {
    Rect,
    Text,
}

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum BindGroupType {
    GlobalUniforms = 0,
    ObjectUniforms = 1,
}

pub struct BindGroupState {
    pub bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
}

pub struct RenderStorage {
    // Quad buffers
    pub unit_quad_vbuf: Buffer,
    pub unit_quad_ibuf: Buffer,

    pub object_buffer: Buffer,
    pub bind_groups: HashMap<BindGroupType, BindGroupState>,
    pipelines: HashMap<PipelineType, RenderPipeline>,
    // Text
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub atlas: TextAtlas,
    pub text_renderer: TextRenderer,
    pub viewport: Viewport,
}

impl RenderStorage {
    pub fn new(state: &WGPUState) -> Self {
        let (unit_quad_vbuf, unit_quad_ibuf, object_buffer) = RenderStorage::create_buffers(&state);
        let bind_groups = RenderStorage::create_bind_groups(&state, &object_buffer);
        let pipelines = RenderStorage::create_pipelines(&state, &bind_groups);

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&state.device);
        let mut viewport = Viewport::new(&state.device, &cache);
        viewport.update(
            &state.queue,
            Resolution {
                width: state.size.0,
                height: state.size.1,
            },
        );
        let mut atlas = TextAtlas::new(
            &state.device,
            &state.queue,
            &cache,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let text_renderer =
            TextRenderer::new(&mut atlas, &state.device, MultisampleState::default(), None);

        Self {
            unit_quad_vbuf,
            unit_quad_ibuf,
            pipelines,
            bind_groups: bind_groups,
            object_buffer,
            swash_cache,
            font_system,
            atlas,
            text_renderer,
            viewport,
        }
    }

    pub fn prepare_text_renderer(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        text_areas: Vec<TextArea>,
    ) {
        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .expect("Failed to prepare text");
    }

    fn create_buffers(state: &WGPUState) -> (Buffer, Buffer, Buffer) {
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

        let quad_vbuf = state.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let quad_ibuf = state.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let object_buffer = state.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Object Buffer"),
            size: (OBJECT_CAPACITY * std::mem::size_of::<RectInstance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        (quad_vbuf, quad_ibuf, object_buffer)
    }

    fn create_pipelines(
        state: &WGPUState,
        bind_groups: &HashMap<BindGroupType, BindGroupState>,
    ) -> HashMap<PipelineType, RenderPipeline> {
        let mut pipeline_hashmap: HashMap<PipelineType, RenderPipeline> = Default::default();

        let quad_pipeline = RenderStorage::create_quad_pipeline(state, &bind_groups);
        pipeline_hashmap.insert(PipelineType::Rect, quad_pipeline);

        pipeline_hashmap
    }

    fn create_quad_pipeline(
        state: &WGPUState,
        bind_groups: &HashMap<BindGroupType, BindGroupState>,
    ) -> RenderPipeline {
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x2, // vertex xy
            ],
        };

        let shader_source = include_str!("../../shaders/quad.wgsl");
        let pipeline = state.create_pipeline(
            "Quad Pipeline",
            shader_source,
            &[vertex_buffer_layout],
            // TODO, really ugly way to do this...
            &[
                &bind_groups
                    .get(&BindGroupType::GlobalUniforms)
                    .expect("No bindgroups. GLOBAL")
                    .bind_group_layout,
                &bind_groups
                    .get(&BindGroupType::ObjectUniforms)
                    .expect("No bindgroups. OBJECT")
                    .bind_group_layout,
            ],
        );

        pipeline
    }

    fn create_bind_groups(
        state: &WGPUState,
        object_buffer: &Buffer,
    ) -> HashMap<BindGroupType, BindGroupState> {
        let mut bind_group_hashmap: HashMap<BindGroupType, BindGroupState> = Default::default();

        let (global_uniform, global_uniform_layout) = RenderStorage::create_global_uniform(&state);
        bind_group_hashmap.insert(
            BindGroupType::GlobalUniforms,
            BindGroupState {
                bind_group: global_uniform,
                bind_group_layout: global_uniform_layout,
            },
        );

        let (object_uniform, object_uniform_layout) =
            RenderStorage::create_object_uniform(&state, object_buffer);
        bind_group_hashmap.insert(
            BindGroupType::ObjectUniforms,
            BindGroupState {
                bind_group: object_uniform,
                bind_group_layout: object_uniform_layout,
            },
        );

        bind_group_hashmap
    }

    fn create_global_uniform(state: &WGPUState) -> (BindGroup, BindGroupLayout) {
        let uniforms = Uniforms {
            viewport: [state.size.0 as f32, state.size.1 as f32],
            _padding: [0.0; 2],
        };

        let uniform_buffer = state.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            state.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = state.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        (uniform_bind_group, uniform_bind_group_layout)
    }

    fn create_object_uniform(
        state: &WGPUState,
        object_buffer: &Buffer,
    ) -> (BindGroup, BindGroupLayout) {
        let object_bind_group_layout =
            state.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("PerObjectDataLayout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let object_bind_group = state.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &object_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: object_buffer.as_entire_binding(),
            }],
            label: Some("PerObjectData"),
        });

        (object_bind_group, object_bind_group_layout)
    }

    pub fn get_pipeline(&self, pipeline_type: &PipelineType) -> &RenderPipeline {
        self.pipelines
            .get(pipeline_type)
            .expect("Piplelines are not initialized")
    }
}
