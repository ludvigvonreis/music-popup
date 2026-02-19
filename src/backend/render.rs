use crate::{
    backend::{
        render_context::{DrawList, RenderContext},
        render_storage::{PipelineType, RenderStorage},
        state::WGPUState,
    },
    types::{ObjectData, Vertex},
    view::{IntoView, View},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::render_storage::BindGroupType;

pub struct Renderer {
    storage: Rc<RefCell<RenderStorage>>,
    state: WGPUState,

    instance_data: Vec<ObjectData>,
}

impl Renderer {
    pub fn new(state: WGPUState) -> Self {
        let storage = RenderStorage::new(&state);

        Self {
            storage: Rc::new(RefCell::new(storage)),
            state,
            instance_data: Default::default(),
        }
    }

    pub fn render_frame(&mut self, root_view: impl IntoView) {
        let mut ctx = RenderContext::default();

        root_view.into_view().paint(&mut ctx);

        // Update instance data.
        self.instance_data.clear();
        let instances = &ctx.draw_list().instanced_data;
        for instance in instances {
            self.instance_data.push(ObjectData {
                position: [instance.pos.x, instance.pos.y],
                size: [instance.size.x, instance.size.y],
                color: [
                    instance.color.r as f32,
                    instance.color.g as f32,
                    instance.color.b as f32,
                    instance.color.a as f32,
                ],
                radius: instance.radius,
                _padding: [0.0; 3],
            });
        }

        // Update buffers with new data.
        {
            let storage = self.storage.as_ref().borrow_mut();

            // let s1: &[u8] = bytemuck::cast_slice(&self.instance_data);
            // println!("{:?}", s1);
            // let s2: &[u8] = bytemuck::cast_slice(&ctx.draw_list().indices);

            self.state.write_buffer(
                &storage.vertex_buffer.clone(),
                0,
                bytemuck::cast_slice(&ctx.draw_list().vertices),
            );

            self.state.write_buffer(
                &storage.index_buffer.clone(),
                0,
                bytemuck::cast_slice(&ctx.draw_list().indices),
            );
            self.state.write_buffer(
                &storage.object_buffer.clone(),
                0,
                bytemuck::cast_slice(&self.instance_data),
            );
        }
        let storage = self.storage.borrow();

        let vertex_buffer = storage.vertex_buffer.clone();
        let index_buffer = storage.index_buffer.clone();
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
                let pipeline = short_lived_storage.get_pipeline(&command.pipeline);

                match command.pipeline {
                    PipelineType::Rect => {
                        pass.set_pipeline(pipeline);

                        pass.set_bind_group(
                            BindGroupType::GlobalUniforms as u32,
                            &uniform_buffer.bind_group,
                            &[],
                        );
                        pass.set_bind_group(
                            BindGroupType::ObjectUniforms as u32,
                            &object_buffer.bind_group,
                            &[],
                        );

                        pass.set_vertex_buffer(
                            0,
                            vertex_buffer.slice(command.vertex_range.clone()),
                        );
                        pass.set_index_buffer(
                            index_buffer.slice(command.index_range.clone()),
                            wgpu::IndexFormat::Uint16,
                        );

                        pass.draw_indexed(0..6, command.vertex_range.start as i32, 0..20);
                    }
                    PipelineType::Text => {}
                }
            }
        });
    }
}
