use std::{io::BufReader, fs::File};

use anyhow::Result;
use ctx::MyVertex;
use obj::{Obj, load_obj};
use vulkano::{memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryUsage}, buffer::{Buffer, BufferCreateInfo, BufferUsage}};

mod ctx;
fn main() -> Result<()> {
    env_logger::init();

    let device_ctx = ctx::DeviceCtx::new()?;

    let suzanne_box: Obj<obj::Vertex, u16> = load_obj(BufReader::new(File::open("in/suzanne_box.obj")?))?;
    let memory_allocator = StandardMemoryAllocator::new_default(device_ctx.device.clone());
    let vertex_buffer = Buffer::from_iter(
        &memory_allocator,
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            usage: MemoryUsage::Upload,
            ..Default::default()
        },
        suzanne_box.indices.iter()
        .map(|index| *suzanne_box.vertices.get(*index as usize).unwrap())
        .map(|vertex| MyVertex { position: vertex.position, normal: vertex.normal }),
    ).unwrap();

    let present_ctx = ctx::PresentCtx::new(&device_ctx, &vertex_buffer)?;

    present_ctx.run(device_ctx, vertex_buffer);
    Ok(())
}


mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 460

            layout(push_constant) uniform PushConstants {
                mat4 view_proj;
            } push_constants;

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 normal;

            layout(location = 0) out vec3 out_normal;

            void main() {
                gl_Position = push_constants.view_proj * vec4(position, 1.0);
                out_normal = normal;
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 460

            layout(location = 0) in vec3 in_normal;

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(normalize(in_normal) / 2.0 + 0.5, 1.0);
            }
        ",
    }
}