use std::ptr::NonNull;

use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use wayland_client::{
    Proxy,
    protocol::{wl_display::WlDisplay, wl_surface::WlSurface},
};
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, RenderPass, RenderPipeline, SurfaceTargetUnsafe,
    VertexBufferLayout, util::DeviceExt,
};

pub struct WGPUState {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,
    pub size: (u32, u32),
}

impl WGPUState {
    pub async fn new(display: &WlDisplay, surface: &WlSurface, size: (u32, u32)) -> Self {
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
            .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0,
            height: size.1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps
                .alpha_modes
                .iter()
                .copied()
                .find(|m| *m == wgpu::CompositeAlphaMode::PreMultiplied)
                .unwrap_or(surface_caps.alpha_modes[0]),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        WGPUState {
            surface,
            device,
            queue,
            config,
            surface_format,
            size,
        }
    }

    pub fn create_pipeline<'a>(
        &self,
        pipeline_label: &str,
        shader_source: &str,
        buffers: &'a [VertexBufferLayout<'a>],
        bind_group_layouts: &[&BindGroupLayout],
    ) -> RenderPipeline {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(shader_source.into()),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(pipeline_label),
                    bind_group_layouts: &bind_group_layouts,
                    immediate_size: 0,
                });

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(pipeline_label),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"), // 1.
                    buffers: &buffers,            // 2.
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    // 3.
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        // 4.
                        format: self.config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One, // src is premultiplied
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw, // 2.
                    cull_mode: None,
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None, // 1.
                multisample: wgpu::MultisampleState {
                    count: 1,                         // 2.
                    mask: !0,                         // 3.
                    alpha_to_coverage_enabled: false, // 4.
                },
                multiview_mask: None, // 5.
                cache: None,          // 6.
            })
    }

    pub fn create_buffer_init(
        &self,
        descriptor: &wgpu::util::BufferInitDescriptor,
    ) -> wgpu::Buffer {
        self.device.create_buffer_init(&descriptor)
    }

    pub fn create_buffer(&self, descriptor: &wgpu::BufferDescriptor) -> wgpu::Buffer {
        self.device.create_buffer(&descriptor)
    }

    pub fn write_buffer(&self, buffer: &Buffer, offset: u64, data: &[u8]) {
        self.queue.write_buffer(buffer, offset, data);
    }

    pub fn create_bind_group_layout(
        &self,
        descriptor: &wgpu::BindGroupLayoutDescriptor<'_>,
    ) -> BindGroupLayout {
        self.device.create_bind_group_layout(descriptor)
    }

    pub fn create_bind_group(&self, descriptor: &wgpu::BindGroupDescriptor<'_>) -> BindGroup {
        self.device.create_bind_group(descriptor)
    }

    pub fn render<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut RenderPass),
    {
        // Create texture view
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                //format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let color: wgpu::Color = wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };

        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Create the renderpass which will clear the screen.
        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        f(&mut renderpass);

        // End the renderpass.
        drop(renderpass);

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        surface_texture.present();
    }
}
