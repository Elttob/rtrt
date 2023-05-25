use std::{sync::Arc, fs::File, rc::Rc};

use anyhow::{Result, bail};
use ash::{vk::{Extent2D, Fence, PipelineStageFlags, SubmitInfo, PresentInfoKHR, CommandBufferBeginInfo, RenderPassBeginInfo, Rect2D, Offset2D, ClearValue, ClearColorValue, SubpassContents, PipelineBindPoint, FenceCreateFlags, self}, util::read_spv};
use winit::{window::Window};

use crate::vulkan::{entry::EntryCtx, instance::InstanceCtx, debug::{DebugCtx, MessageSeverityFlags, MessageTypeFlags}, surface::SurfaceCtx, device::DeviceCtx, swapchain::SwapchainCtx, shader::ShaderCtx, render_pass::RenderPassCtx, pipeline::PipelineCtx, framebuffer::{FramebufferCtx}, semaphore::SemaphoreCtx, fence::FenceCtx};

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

        framebuffer.fb_command_buffers.buffers.iter()
            .zip(framebuffer.framebuffers.iter())
            .map(|(buffer, framebuffer)| {
                let buffer = *buffer;
                let command_buffer_begin_info = CommandBufferBeginInfo::builder().build();
                unsafe { device.logical_info.device.begin_command_buffer(buffer, &command_buffer_begin_info)? };
                let clear_values = [ClearValue {
                    color: ClearColorValue {
                        float32: [0.0, 1.0, 0.0, 1.0],
                    }
                }];
                let render_pass_begin_info = RenderPassBeginInfo::builder()
                    .render_pass(render_pass.render_pass)
                    .framebuffer(*framebuffer)
                    .render_area(Rect2D {
                        offset: Offset2D { x: 0, y: 0 },
                        extent
                    })
                    .clear_values(&clear_values)
                    .build();

                unsafe { 
                    device.logical_info.device.cmd_begin_render_pass(buffer, &render_pass_begin_info, SubpassContents::INLINE);
                    device.logical_info.device.cmd_bind_pipeline(buffer, PipelineBindPoint::GRAPHICS, pipeline.pipeline);
                    device.logical_info.device.cmd_draw(buffer, 3, 1, 0, 0);
                    device.logical_info.device.cmd_end_render_pass(buffer);
                    device.logical_info.device.end_command_buffer(buffer)?;
                }
                Ok(())
            })
            .collect::<Result<_>>()?;

        Ok(SizedRenderer {
            swapchain,
            render_pass,
            pipeline,
            framebuffer
        })
    }
}

pub struct Renderer {
    window: Arc<Window>,
    entry: Rc<EntryCtx>,
    instance: Rc<InstanceCtx>,
    debug: Rc<DebugCtx>,
    surface: Rc<SurfaceCtx>,
    device: Rc<DeviceCtx>,
    shader: Rc<ShaderCtx>,
    sized_renderer: Option<SizedRenderer>,
    resize_needed: bool,
    max_frames_in_flight: usize,
    current_frame_in_flight: usize,
    image_available_semaphores: Vec<Rc<SemaphoreCtx>>,
    render_finished_semaphores: Vec<Rc<SemaphoreCtx>>,
    in_flight_fences: Vec<Rc<FenceCtx>>
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
        let surface = SurfaceCtx::new(instance.clone(), window.clone())?;
        let device = DeviceCtx::new(surface.clone())?;
        let shader = ShaderCtx::new(device.clone(), 
            &read_spv(&mut File::open("in/spirv/shaders.spv")?)?, 
            "shaders.spv".to_string()
        )?;

        let max_frames_in_flight = 2;
        let image_available_semaphores = (0..max_frames_in_flight)
            .map(|_| SemaphoreCtx::new(device.clone()))
            .collect::<Result<Vec<_>>>()?;
        let render_finished_semaphores = (0..max_frames_in_flight)
            .map(|_| SemaphoreCtx::new(device.clone()))
            .collect::<Result<Vec<_>>>()?;
        let in_flight_fences = (0..max_frames_in_flight)
            .map(|_| FenceCtx::new(device.clone(), FenceCreateFlags::SIGNALED))
            .collect::<Result<Vec<_>>>()?;

        let mut me = Renderer {
            window,
            entry,
            instance,
            debug,
            surface,
            device,
            shader,
            sized_renderer: None,
            resize_needed: true,
            max_frames_in_flight,
            current_frame_in_flight: 0,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences
        };
        me.recreate_sized_renderer()?;
        Ok(me)
    }

    pub fn resize(
        &mut self
    ) {
        self.resize_needed = true;
    }

    fn recreate_sized_renderer(
        &mut self
    ) -> Result<()> {
        log::debug!("Will recreate sized renderer at next opportunity...");
        self.device.wait_for_idle()?;
        let inner_size = self.window.inner_size();
        if inner_size.width <= 0 || inner_size.height <= 0 {
            log::debug!("Window has zero size ({} x {}), not recreating sized renderer", inner_size.width, inner_size.height);
            self.sized_renderer = None;
        } else {
            log::debug!("Recreating sized renderer ({} x {})...", inner_size.width, inner_size.height);
            self.sized_renderer = Some(SizedRenderer::new(self.device.clone(), self.shader.clone(), 
                Extent2D {
                    width: self.window.inner_size().width,
                    height: self.window.inner_size().height
                }
            )?);
        }
        self.resize_needed = false;
        Ok(())
    }

    pub fn render(
        &mut self
    ) -> Result<()> {
        log::debug!("Rendering...");
        let sized_renderer = match &mut self.sized_renderer {
            Some(renderer) => renderer,
            None => return Ok(())
        };
        let swapchain = &sized_renderer.swapchain;
        let swapchains_khr = [swapchain.swapchain_khr];
        let framebuffer = &sized_renderer.framebuffer;

        let image_available_semaphore = self.image_available_semaphores[self.current_frame_in_flight].semaphore;
        let render_finished_semaphore = self.render_finished_semaphores[self.current_frame_in_flight].semaphore;
        let in_flight_fence = self.in_flight_fences[self.current_frame_in_flight].fence;
        let command_buffer = framebuffer.fb_command_buffers.buffers[self.current_frame_in_flight];
        let image_available_semaphores = [image_available_semaphore];
        let render_finished_semaphores = [render_finished_semaphore];
        let in_flight_fences = [in_flight_fence];
        let command_buffers = [command_buffer];

        // This fence is reset (i.e. will block) when a previous render() call decides to submit work.
        // Waiting for it here ensures we don't override that render() call's work.
        unsafe { self.device.logical_info.device.wait_for_fences(&in_flight_fences, true, std::u64::MAX)? };

        let acquire_result = unsafe { swapchain.swapchain.acquire_next_image(
            swapchain.swapchain_khr, std::u64::MAX, image_available_semaphore, Fence::null()
        ) };

        // TODO Oh hey this might be a problem. The command buffer won't target this image index.
        let image_index = match acquire_result {
            Ok((image_index, _suboptimal)) => image_index,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_sized_renderer()?;
                return Ok(());
            },
            Err(e) => return Err(e.into())
        };
        let images_indices = [image_index];

        // We should only reset the fence when we're actually going to submit work
        // that will re-signal it later on, otherwise it'll hang forever.
        unsafe { self.device.logical_info.device.reset_fences(&in_flight_fences)? };
        
        { // render
            let wait_stages = [PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let submit_info = SubmitInfo::builder()
                .wait_semaphores(&image_available_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&render_finished_semaphores)
                .build();
            let submit_infos = [submit_info];
            unsafe { self.device.logical_info.device.queue_submit(
                self.device.logical_info.graphics_queue, 
                &submit_infos, 
                in_flight_fence
            )? };
        }
        { // present
            let present_info = PresentInfoKHR::builder()
                .wait_semaphores(&render_finished_semaphores)
                .swapchains(&swapchains_khr)
                .image_indices(&images_indices)
                .build();
            let present_result = unsafe { swapchain.swapchain.queue_present(self.device.logical_info.present_queue, &present_info) };
            let needs_rebuild = match present_result {
                Ok(suboptimal) => suboptimal | self.resize_needed,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => true,
                Err(e) => bail!(e)
            };
            if needs_rebuild {
                self.recreate_sized_renderer()?;
            }
            // self.current_frame_in_flight = (self.current_frame_in_flight + 1) % self.max_frames_in_flight;
            Ok(())
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        log::debug!("Dropping renderer after work is completed...");
        self.device.wait_for_idle().unwrap();
    }
}