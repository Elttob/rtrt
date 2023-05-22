use ash::vk::{Framebuffer, FramebufferCreateInfo};
use anyhow::{Result};

use super::render_pass::RenderPassCtx;

pub struct FramebufferCtx<'ren, 'swp, 'dev, 'srf, 'ins, 'en> {
    pub render_pass_ctx: &'ren RenderPassCtx<'swp, 'dev, 'srf, 'ins, 'en>,
    pub framebuffers: Vec<Framebuffer>
}

impl<'swp, 'dev, 'srf, 'ins, 'en> RenderPassCtx<'swp, 'dev, 'srf, 'ins, 'en> {
    pub fn create_framebuffer_ctx(
        &self,
    ) -> Result<FramebufferCtx> {
        let framebuffers = self.swapchain_ctx.image_views.iter()
            .map(|view| [*view])
            .map(|attachments| {
                let framebuffer_info = FramebufferCreateInfo::builder()
                    .render_pass(self.render_pass)
                    .attachments(&attachments)
                    .width(self.swapchain_ctx.swapchain_extent.width)
                    .height(self.swapchain_ctx.swapchain_extent.height)
                    .layers(1)
                    .build();
                Ok(unsafe { self.swapchain_ctx.device_ctx.logical_info.device.create_framebuffer(&framebuffer_info, None)? })
            })
            .collect::<Result<Vec<_>>>()?;

        log::debug!("FramebufferCtx created");
        Ok(FramebufferCtx {
            render_pass_ctx: self,
            framebuffers
        })
    }
}

impl Drop for FramebufferCtx<'_, '_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        unsafe {
            for framebuffer in self.framebuffers {
                self.render_pass_ctx.swapchain_ctx.device_ctx.logical_info.device.destroy_framebuffer(framebuffer, None);
            }
        }
        log::debug!("FramebufferCtx dropped");
    }
}