use std::{sync::Arc, fs::File};

use anyhow::Result;
use ash::{Entry, vk::Extent2D, util::read_spv};
use winit::{window::{WindowBuilder, Window}, event_loop::EventLoop};

use crate::ctx::{debug::{MessageSeverityFlags, MessageTypeFlags}, entry::EntryCtx};

mod ctx;

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

    let (_event_loop, window) = create_window()?;
    let preferred_extent = Extent2D {
        width: window.inner_size().width,
        height: window.inner_size().height
    };

    let entry_ctx = EntryCtx { entry: Entry::linked() };
    let instance_ctx = entry_ctx.create_instance_ctx(Default::default(), &[], true)?;
    let _debug_ctx = instance_ctx.create_debug_ctx(
        MessageSeverityFlags { warning: true, error: true, ..Default::default() },
        MessageTypeFlags { validation: true, ..Default::default() }
    )?;
    let surface_ctx = instance_ctx.create_surface_ctx(window.clone())?;
    let device_ctx = surface_ctx.create_device_ctx()?;
    let swapchain_ctx = device_ctx.create_swapchain_ctx(preferred_extent)?;
    let shader_ctx = device_ctx.create_shader_ctx(&read_spv(&mut File::open(env!("shaders.spv"))?)?);
    
    // TODO: 1.2.1 here -> https://github.com/adrien-ben/vulkan-tutorial-rs/commits/master?after=6c47737e505aa7b2b5a4d7b2711490b2482c246b+34&branch=master&qualified_name=refs%2Fheads%2Fmaster

    Ok(())
}