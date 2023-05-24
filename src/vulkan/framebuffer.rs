use std::rc::Rc;

use ash::vk::{Framebuffer, FramebufferCreateInfo, CommandBufferLevel};
use anyhow::{Result};

use super::{render_pass::RenderPassCtx, command_pool::CommandPoolCtx, command_buffers::CommandBuffersCtx};

pub struct FramebufferCtx {
    pub render_pass_ctx: Rc<RenderPassCtx>,
    pub framebuffers: Vec<Framebuffer>,
    pub fb_command_pool: Rc<CommandPoolCtx>,
    pub fb_command_buffers: Rc<CommandBuffersCtx>,
}

impl FramebufferCtx {
    pub fn new(
        render_pass_ctx: Rc<RenderPassCtx>
    ) -> Result<FramebufferCtx> {
        let framebuffers = render_pass_ctx.swapchain_ctx.image_views.iter()
            .map(|view| [*view])
            .map(|attachments| {
                let framebuffer_info = FramebufferCreateInfo::builder()
                    .render_pass(render_pass_ctx.render_pass)
                    .attachments(&attachments)
                    .width(render_pass_ctx.swapchain_ctx.swapchain_extent.width)
                    .height(render_pass_ctx.swapchain_ctx.swapchain_extent.height)
                    .layers(1)
                    .build();
                Ok(unsafe { render_pass_ctx.swapchain_ctx.device_ctx.logical_info.device.create_framebuffer(&framebuffer_info, None)? })
            })
            .collect::<Result<Vec<_>>>()?;

        let fb_command_pool = CommandPoolCtx::new(render_pass_ctx.swapchain_ctx.device_ctx.clone())?;
        let fb_command_buffers = CommandBuffersCtx::new(fb_command_pool.clone(), CommandBufferLevel::PRIMARY, framebuffers.len() as u32)?;

        log::debug!("FramebufferCtx created");
        Ok(FramebufferCtx {
            render_pass_ctx,
            framebuffers,
            fb_command_pool,
            fb_command_buffers
        })
    }
}

impl Drop for FramebufferCtx {
    fn drop(&mut self) {
        unsafe {
            for framebuffer in self.framebuffers.iter() {
                self.render_pass_ctx.swapchain_ctx.device_ctx.logical_info.device.destroy_framebuffer(*framebuffer, None);
            }
        }
        log::debug!("FramebufferCtx dropped");
    }
}