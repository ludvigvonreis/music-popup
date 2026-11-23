use glyphon::{TextArea, TextBounds};

use crate::{
    backend::{
        layout_context::LayoutContext,
        render_context::{ClipRect, DrawCommand, DrawKind, RenderContext},
        render_storage::RenderStorage,
        state::WGPUState,
    },
    view::{IntoView, View},
};
use std::{cell::RefCell, rc::Rc};

use super::render_storage::BindGroupType;

pub struct Renderer {
    storage: Rc<RefCell<RenderStorage>>,
    state: WGPUState,

    root_view: Box<dyn View>,
}

impl Renderer {
    pub fn new(state: WGPUState, root_view: impl IntoView) -> Self {
        let storage = RenderStorage::new(&state);

        Self {
            storage: Rc::new(RefCell::new(storage)),
            state,
            root_view: Box::new(root_view.into_view()),
        }
    }

    pub fn layout_frame(&mut self) {
        let mut ctx = LayoutContext::new(self.storage.clone());
        let (w, h) = self.state.size;
        crate::view::layout::compute(self.root_view.id(), (w as f32, h as f32), &mut ctx);
        self.root_view.layout(&mut ctx); // post-pass: views read their rects
    }

    pub fn render_frame(&mut self) {
        let mut ctx = RenderContext::new(self.storage.clone());

        self.root_view.paint(&mut ctx);

        // Update instance data.
        let rect_instances = ctx.draw_list().rect_instances.clone();
        let (vw, vh) = self.state.size;
        let dl = ctx.draw_list();
        let text_areas: Vec<TextArea> = dl
            .text_instances
            .iter()
            .zip(&dl.text_clips)
            .map(|(e, clip)| TextArea {
                buffer: &e.buffer,
                left: e.x,
                top: e.y,
                scale: 1.0,
                bounds: match clip {
                    Some(c) => TextBounds {
                        left: c.x as i32,
                        top: c.y as i32,
                        right: (c.x + c.width) as i32,
                        bottom: (c.y + c.height) as i32,
                    },
                    None => TextBounds {
                        left: 0,
                        top: 0,
                        right: vw as i32,
                        bottom: vh as i32,
                    },
                },
                default_color: e.color.into(),
                custom_glyphs: &[],
            })
            .collect();

        // Update buffers with new data.
        {
            let mut storage = self.storage.as_ref().borrow_mut();
            storage.prepare_text_renderer(&self.state.queue, &self.state.device, text_areas);

            self.state.write_buffer(
                &storage.object_buffer.clone(),
                0,
                bytemuck::cast_slice(&rect_instances),
            );

            // Image buffers
            let image_instances = ctx.draw_list().image_instances.clone();
            storage.ensure_images(&self.state, &ctx.draw_list().image_assets);
            self.state.write_buffer(
                &storage.image_instance_buffer.clone(),
                0,
                bytemuck::cast_slice(&image_instances),
            );
        }
        let storage = self.storage.borrow();

        let unit_quad_vbuf = storage.unit_quad_vbuf.clone();
        let unit_quad_ibuf = storage.unit_quad_ibuf.clone();
        let uniform_buffer = storage
            .bind_groups
            .get(&BindGroupType::GlobalUniforms)
            .expect("Bind group not set");
        let object_buffer = storage
            .bind_groups
            .get(&BindGroupType::ObjectUniforms)
            .expect("Bind group not set");

        let image_instance_buffer = storage.image_instance_buffer.clone();

        let commands = &ctx.draw_list().commands;
        let mut sorted_commands: Vec<&DrawCommand> = commands.iter().collect();

        sorted_commands.sort_by_key(|cmd| match &cmd.kind {
            DrawKind::Rects { .. } => 0,
            DrawKind::Images { .. } => 1,
            DrawKind::Glyphs => 2,
        });

        self.state.render(|pass| {
            for command in &sorted_commands {
                let short_lived_storage = self.storage.borrow();
                let clip = match &command.kind {
                    // Text is clipped through TextArea.bounds instead.
                    DrawKind::Glyphs => None,
                    _ => command.clip,
                };
                let c = clip
                    .unwrap_or(ClipRect {
                        x: 0,
                        y: 0,
                        width: vw,
                        height: vh,
                    })
                    .clamp_to(vw, vh);
                if c.width == 0 || c.height == 0 {
                    continue; // fully clipped away
                }
                pass.set_scissor_rect(c.x, c.y, c.width, c.height);

                match &command.kind {
                    DrawKind::Rects { instance_range } => {
                        let pipeline = short_lived_storage.get_pipeline(&command.pipeline);
                        pass.set_pipeline(pipeline);

                        // Contains viewport size.
                        pass.set_bind_group(
                            BindGroupType::GlobalUniforms as u32,
                            &uniform_buffer.bind_group,
                            &[],
                        );

                        // Contains instanced data as uniform. should be changed.
                        pass.set_bind_group(
                            BindGroupType::ObjectUniforms as u32,
                            &object_buffer.bind_group,
                            &[],
                        );

                        pass.set_vertex_buffer(0, unit_quad_vbuf.slice(..));
                        pass.set_index_buffer(unit_quad_ibuf.slice(..), wgpu::IndexFormat::Uint16);

                        pass.draw_indexed(0..6, 0, instance_range.clone());
                    }
                    DrawKind::Glyphs => {
                        storage
                            .text_renderer
                            .render(&storage.atlas, &storage.viewport, pass)
                            .expect("Failed to render text");
                    }
                    DrawKind::Images {
                        image_id,
                        instance_range,
                    } => {
                        let Some(image_bg) = short_lived_storage.images.get(image_id) else {
                            continue;
                        };
                        pass.set_pipeline(short_lived_storage.get_pipeline(&command.pipeline));
                        pass.set_bind_group(0, &uniform_buffer.bind_group, &[]);
                        pass.set_bind_group(1, image_bg, &[]);
                        pass.set_vertex_buffer(0, unit_quad_vbuf.slice(..));
                        pass.set_vertex_buffer(1, image_instance_buffer.slice(..));
                        pass.set_index_buffer(unit_quad_ibuf.slice(..), wgpu::IndexFormat::Uint16);
                        pass.draw_indexed(0..6, 0, instance_range.clone());
                    }
                }
            }
        });
    }
}
