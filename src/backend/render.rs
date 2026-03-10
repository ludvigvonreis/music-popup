use glyphon::{TextArea, TextBounds, Viewport};

use crate::{
    backend::{
        layout_context::LayoutContext,
        render_context::{DrawKind, RenderContext},
        render_storage::RenderStorage,
        state::WGPUState,
    },
    types::RectInstance,
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
        self.root_view.layout(&mut ctx);
    }

    pub fn render_frame(&mut self) {
        let mut ctx = RenderContext::new(self.storage.clone());

        self.root_view.paint(&mut ctx);

        // Update instance data.
        let rect_instances = ctx.draw_list().rect_instances.clone();
        let text_areas: Vec<TextArea> = ctx
            .draw_list()
            .text_instances
            .iter()
            .map(|e| TextArea {
                buffer: &e.buffer,
                left: e.x,
                top: e.y,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: 9999,
                    bottom: 9999,
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

        let commands = &ctx.draw_list().commands;
        self.state.render(|pass| {
            for command in commands {
                let short_lived_storage = self.storage.borrow();

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
                }
            }
        });
    }
}
