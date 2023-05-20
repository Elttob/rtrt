use std::{io::BufReader, fs::File, sync::Arc};

use anyhow::Result;
use glam::vec2;
use input::Input;
use obj::load_obj;
use scene::Scene;
use winit::{window::WindowBuilder, event_loop::{EventLoop, ControlFlow}, event::{Event, WindowEvent}};

mod ctx;
mod input;
mod scene;

fn main() -> Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Real Time Ray Tracing")
        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
        .with_visible(false)
        .build(&event_loop)?;
    let monitor = window.current_monitor().unwrap_or(
        window.primary_monitor().unwrap_or(
            window.available_monitors().next().expect("Couldn't find a suitable monitor.")
        )
    );
    let monitor_size = monitor.size();
    let window_size = window.outer_size();
    window.set_outer_position(winit::dpi::PhysicalPosition::new(
        (monitor_size.width - window_size.width) / 2,
        (monitor_size.height - window_size.height) / 2
    ));
    let window = Arc::new(window);

    let device_ctx = ctx::DeviceCtx::new(window.clone())?;
    let scene = Scene::from_obj(&device_ctx, load_obj(BufReader::new(File::open("in/suzanne_box.obj")?))?)?;
    let mut present_ctx = ctx::PresentCtx::new(&device_ctx, window.inner_size())?;
    
    {
        let mut cursor_over_window = false;
        let mut input = Input::new(vec2(0.0, 0.0));

        window.set_visible(true);
        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    present_ctx.recreate_swapchain(&device_ctx, *physical_size).unwrap();
                },
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    present_ctx.recreate_swapchain(&device_ctx, **new_inner_size).unwrap();
                },
                WindowEvent::CursorEntered { .. } => {
                    cursor_over_window = true;
                },
                WindowEvent::CursorLeft { .. } => {
                    cursor_over_window = false;
                },
                _ => {
                    if window.has_focus() && cursor_over_window {
                        input.process_window_events(event);
                    }
                }
            },
            Event::DeviceEvent { ref event, .. } => {
                if window.has_focus() && cursor_over_window {
                    input.process_device_events(event);
                    let window_size = window.inner_size();
                    window.set_cursor_position(winit::dpi::PhysicalPosition::new(
                        window_size.width / 2,
                        window_size.height / 2
                    )).expect("Platform does not support setting the cursor position");
                }
            },
            Event::MainEventsCleared => {
                present_ctx.render(&device_ctx, &scene).unwrap();
            }
            _ => (),
        });
    }
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 460

            layout(push_constant) uniform PushConstants {
                mat4 proj;
                mat4 view;
            } push_constants;

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 normal;

            layout(location = 0) out vec3 out_normal;

            void main() {
                gl_Position = push_constants.proj * push_constants.view * vec4(position, 1.0);
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