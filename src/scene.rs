use std::sync::Arc;

use anyhow::Result;
use obj::Obj;
use vulkano::{pipeline::{graphics::vertex_input::Vertex, GraphicsPipeline, Pipeline}, buffer::{BufferContents, Subbuffer, Buffer, BufferCreateInfo, BufferUsage}, memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryUsage}, command_buffer::{allocator::StandardCommandBufferAllocator, PrimaryAutoCommandBuffer, AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents}, device::Queue, render_pass::Framebuffer};

use crate::ctx::DeviceCtx;

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct GeoVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub normal: [f32; 3]
}

pub struct Scene {
    geometry: Subbuffer<[GeoVertex]>
}

impl Scene {
    pub fn from_obj(
        device_ctx: &DeviceCtx,
        obj: Obj<obj::Vertex, u16>
    ) -> Result<Self> {
        let memory_allocator = StandardMemoryAllocator::new_default(device_ctx.device.clone());
        let geometry = Buffer::from_iter(
            &memory_allocator,
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            obj.indices.iter()
            .map(|index| *obj.vertices.get(*index as usize).unwrap())
            .map(|vertex| GeoVertex { position: vertex.position, normal: vertex.normal }),
        )?;

        Ok(Self {
            geometry
        })
    }

    pub fn build_command_buffer(
        &self,
        command_buffer_allocator: &StandardCommandBufferAllocator,
        queue: &Arc<Queue>,
        pipeline: &Arc<GraphicsPipeline>,
        framebuffer: &Arc<Framebuffer>
    ) -> Arc<PrimaryAutoCommandBuffer> {
        let mut builder = AutoCommandBufferBuilder::primary(
            command_buffer_allocator,
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit
        )
        .unwrap();

        let extents = framebuffer.extent();
        let push_constants = crate::vs::PushConstants {
            proj: 
                (glam::Mat4::from_scale(glam::vec3(1.0, -1.0, 1.0)) * glam::Mat4::perspective_lh(1.5, extents[0] as f32 / extents[1] as f32, 0.1, 100.0))
                .to_cols_array_2d(),
            view: 
                (glam::Mat4::look_at_lh(glam::vec3(1.0, 1.0, 1.0), glam::vec3(0.0, 0.0, 0.0), glam::vec3(0.0, 1.0, 0.0)))
                .to_cols_array_2d()
        };

        builder.begin_render_pass(
            RenderPassBeginInfo {
                clear_values: vec![Some([0.3, 0.5, 0.7, 1.0].into())],
                ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
            },
            SubpassContents::Inline
        )
        .unwrap()
        .bind_pipeline_graphics(pipeline.clone())
        .bind_vertex_buffers(0, self.geometry.clone())
        .push_constants(pipeline.layout().clone(), 0, push_constants)
        .draw(self.geometry.len() as u32, 1, 0, 0)
        .unwrap()
        .end_render_pass()
        .unwrap();

        Arc::new(builder.build().unwrap())
    }
}