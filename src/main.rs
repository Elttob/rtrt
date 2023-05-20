use anyhow::Result;
use vkw::{MessageSeverityFlags, MessageTypeFlags};
use winit::{window::{WindowBuilder, Window}, event_loop::EventLoop};

mod vkw;
mod physical_devices;

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
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    log::debug!("Starting!");

    let (event_loop, window) = create_window()?;

    let entry = ash::Entry::linked();
    let instance = vkw::Instance::new(
        &entry,
        Default::default(),
        &[
            ash::extensions::khr::Surface::name(),
            ash::extensions::khr::Win32Surface::name()
        ],
        Some((MessageSeverityFlags::all(), MessageTypeFlags::all()))
    )?;
    let physical_device = physical_devices::select((&instance).into());
    
    Ok(())
}