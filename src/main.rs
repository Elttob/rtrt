use std::sync::Arc;

use anyhow::Result;
use glam::vec2;
use winit::{window::{WindowBuilder, Window}, event_loop::{EventLoop, ControlFlow}, event::{Event, WindowEvent}};

use crate::{render::Renderer, input::Input};

mod vulkan;
mod input;
mod render;

fn create_window() -> Result<(EventLoop<()>, Arc<Window>)> {
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

    Ok((event_loop, Arc::new(window)))
}

fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "DEBUG");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    log::debug!("Starting!");

    let (event_loop, window) = create_window()?;
    let mut renderer = Renderer::new(window.clone())?;
    
    // TODO: 1.3.3.1 here -> https://github.com/adrien-ben/vulkan-tutorial-rs/commits/master?after=6c47737e505aa7b2b5a4d7b2711490b2482c246b+34&branch=master&qualified_name=refs%2Fheads%2Fmaster

    {
        let mut cursor_over_window = false;
        let mut input = Input::new(vec2(0.0, 0.0));

        window.set_visible(true);
        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(_) => renderer.resize(),
                WindowEvent::ScaleFactorChanged { .. } => renderer.resize(),
                WindowEvent::CursorEntered { .. } => cursor_over_window = true,
                WindowEvent::CursorLeft { .. } => cursor_over_window = true,
                _ => if window.has_focus() && cursor_over_window {
                    input.process_window_events(event);
                }
            },
            Event::DeviceEvent { ref event, .. } => if window.has_focus() && cursor_over_window {
                input.process_device_events(event);
                let window_size = window.inner_size();
                window.set_cursor_position(winit::dpi::PhysicalPosition::new(
                    window_size.width / 2,
                    window_size.height / 2
                )).expect("Platform does not support setting the cursor position");
            },
            Event::MainEventsCleared => {
                // let snapshot = input.snapshot();
                // scene.camera.pitch_yaw_radians = snapshot.pitch_yaw_radians;
                // let move_speed = 2.0 / 144.0;
                // scene.camera.position += snapshot.move_axes.z * move_speed * pitch_yaw::look_dir(scene.camera.pitch_yaw_radians);
                // scene.camera.position += snapshot.move_axes.y * move_speed * Vec3::Y;
                // scene.camera.position += snapshot.move_axes.x * move_speed * pitch_yaw::flat_right_vec(scene.camera.pitch_yaw_radians.y);
                renderer.render().unwrap();
            }
            _ => (),
        });
    }
}