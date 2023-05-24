use std::{sync::Arc, fs::File, rc::Rc};

use anyhow::{Result, Ok, bail};
use ash::{vk::Extent2D, util::read_spv};
use winit::{window::Window};

use crate::vulkan::{entry::EntryCtx, instance::InstanceCtx, debug::{DebugCtx, MessageSeverityFlags, MessageTypeFlags}, surface::SurfaceCtx, device::DeviceCtx, swapchain::SwapchainCtx, shader::ShaderCtx, render_pass::RenderPassCtx, pipeline::PipelineCtx, framebuffer::FramebufferCtx, semaphore::SemaphoreCtx};

struct SizedRenderer {
    swapchain: Rc<SwapchainCtx>,
    render_pass: Rc<RenderPassCtx>,
    pipeline: Rc<PipelineCtx>,
    framebuffer: Rc<FramebufferCtx>
}

impl SizedRenderer {
    pub fn new(
        device: Rc<DeviceCtx>,
        shader: Rc<ShaderCtx>,
        extent: Extent2D
    ) -> Result<SizedRenderer> {
        let swapchain = SwapchainCtx::new(device.clone(), extent)?;
        let render_pass = RenderPassCtx::new(swapchain.clone())?;
        let pipeline = PipelineCtx::new(render_pass.clone(), shader.clone())?;
        let framebuffer = FramebufferCtx::new(render_pass.clone())?;

        Ok(SizedRenderer {
            swapchain,
            render_pass,
            pipeline,
            framebuffer
        })
    }
}

enum SizedRendererState {
    Invalidated,
    ZeroSize,
    Sized(SizedRenderer)
}

pub struct Renderer {
    window: Arc<Window>,
    entry: Rc<EntryCtx>,
    instance: Rc<InstanceCtx>,
    debug: Rc<DebugCtx>,
    surface: Rc<SurfaceCtx>,
    device: Rc<DeviceCtx>,
    shader: Rc<ShaderCtx>,
    image_available_semaphore: Rc<SemaphoreCtx>,
    render_finished_semaphore: Rc<SemaphoreCtx>,
    sized_renderer: SizedRendererState
}

impl Renderer {
    pub fn new(
        window: Arc<Window>
    ) -> Result<Renderer> {
        let entry = EntryCtx::new();
        let instance = InstanceCtx::new(entry.clone(), Default::default(), &[], true)?;
        let debug = DebugCtx::new(instance.clone(), 
            MessageSeverityFlags { warning: true, error: true, ..Default::default() }, 
            MessageTypeFlags { validation: true, ..Default::default() }
        )?;
        let surface = SurfaceCtx::new(instance.clone(), window.clone() )?;
        let device = DeviceCtx::new(surface.clone())?;
        let shader = ShaderCtx::new(device.clone(), 
            &read_spv(&mut File::open("in/spirv/shaders.spv")?)?, 
            "shaders.spv".to_string()
        )?;
        let image_available_semaphore = SemaphoreCtx::new(device.clone())?;
        let render_finished_semaphore = SemaphoreCtx::new(device.clone())?;

        Ok(Renderer {
            window,
            entry,
            instance,
            debug,
            surface,
            device,
            shader,
            image_available_semaphore,
            render_finished_semaphore,
            sized_renderer: SizedRendererState::Invalidated
        })
    }

    pub fn resize(&mut self) {
        self.sized_renderer = SizedRendererState::Invalidated;
    }

    fn recreate_sized_renderer(
        &mut self
    ) -> Result<()> {
        let inner_size = self.window.inner_size();
        if inner_size.width <= 0 || inner_size.height <= 0 {
            log::debug!("Window has zero size ({} x {}), not recreating sized renderer", inner_size.width, inner_size.height);
            self.sized_renderer = SizedRendererState::ZeroSize;
        } else {
            log::debug!("Recreating sized renderer ({} x {})...", inner_size.width, inner_size.height);
            self.sized_renderer = SizedRendererState::Sized(
                SizedRenderer::new(self.device.clone(), self.shader.clone(), Extent2D {
                    width: self.window.inner_size().width,
                    height: self.window.inner_size().height
                })?
            );
        }
        match &self.sized_renderer {
            SizedRendererState::Invalidated => bail!("Couldn't recreate sized renderer for some reason."),
            _ => Ok(())
        }
    }

    pub fn render(
        &mut self
    ) -> Result<()> {
        let sized_renderer = loop {
            match &mut self.sized_renderer {
                SizedRendererState::Sized(renderer) => break renderer,
                SizedRendererState::ZeroSize => return Ok(()),
                SizedRendererState::Invalidated => self.recreate_sized_renderer()?
            };
        };


        Ok(())
    }
}