#![feature(iterator_try_collect)]

use std::{io::BufReader, fs::File};

use anyhow::Result;
use glam::{vec2, vec3};
use obj::load_obj;
use winit::{window::{WindowBuilder, Window}, event_loop::{EventLoop, ControlFlow}, event::{Event, WindowEvent}};

use crate::{render::Renderer, input::Input, scene::Scene};

mod input;
mod pitch_yaw;
mod scene;
mod render;

fn create_window() -> Result<(EventLoop<()>, Window)> {
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

    Ok((event_loop, window))
}

fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "DEBUG");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    log::debug!("Starting!");

    let (event_loop, window) = create_window()?;
    let mut renderer = Renderer::new(&window)?;
    let mut input = Input::new(vec2(0.0, 0.0));
    let mut scene = Scene::from_obj_righthanded(load_obj(BufReader::new(File::open("in/duskroom_simple.obj")?))?)?;
    
    let mut cursor_over_window = false;

    window.set_visible(true);
    event_loop.run(move |event, _target, flow| {
        *flow = ControlFlow::Poll;

        (|| -> Result<()> {
            match event {
                Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
                    WindowEvent::CloseRequested => {
                        renderer.wait_idle()?;
                        *flow = ControlFlow::Exit;
                    },
                    WindowEvent::CursorEntered { .. } => cursor_over_window = true,
                    WindowEvent::CursorLeft { .. } => cursor_over_window = false,
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
                Event::MainEventsCleared => window.request_redraw(),
                Event::RedrawRequested(window_id) if window_id == window.id() => {
                    let snapshot = input.snapshot();
                    scene.camera.pitch_yaw_radians = snapshot.pitch_yaw_radians;
                    let move_speed = 2.0 / 144.0;
                    scene.camera.position += snapshot.move_axes.z * move_speed * pitch_yaw::look_dir(scene.camera.pitch_yaw_radians);
                    scene.camera.position += snapshot.move_axes.y * move_speed * vec3(0.0, 1.0, 0.0);
                    scene.camera.position += snapshot.move_axes.x * move_speed * pitch_yaw::flat_right_vec(scene.camera.pitch_yaw_radians.y);
                    
                    renderer.render()?
                },
                _ => {}
            };
            Ok(())
        })().unwrap()
    });
}