use anyhow::Result;
use ash::vk::{AttachmentDescription, SampleCountFlags, AttachmentLoadOp, ImageLayout, AttachmentReference, SubpassDescription, PipelineBindPoint, AttachmentStoreOp, RenderPassCreateInfo, RenderPass};

use super::swapchain::SwapchainCtx;

pub struct RenderPassCtx<'swp, 'dev, 'srf, 'ins, 'en> {
    pub swapchain_ctx: &'swp SwapchainCtx<'dev, 'srf, 'ins, 'en>,
    pub render_pass: RenderPass
}

impl<'dev, 'srf, 'ins, 'en> SwapchainCtx<'dev, 'srf, 'ins, 'en> {
    pub fn create_render_pass_ctx(
        &self,
    ) -> Result<RenderPassCtx> {
        let attachment_desc = AttachmentDescription::builder()
            .format(self.swapchain_image_format)
            .samples(SampleCountFlags::TYPE_1)
            .load_op(AttachmentLoadOp::CLEAR)
            .store_op(AttachmentStoreOp::STORE)
            .initial_layout(ImageLayout::UNDEFINED)
            .final_layout(ImageLayout::PRESENT_SRC_KHR)
            .build();
        let attachment_descs = [attachment_desc];
        let attachment_ref = AttachmentReference::builder()
            .attachment(0)
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();
        let attachment_refs = [attachment_ref];
        let subpass_desc = SubpassDescription::builder()
            .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
            .color_attachments(&attachment_refs)
            .build();
        let subpass_descs = [subpass_desc];
        let render_pass_info = RenderPassCreateInfo::builder()
            .attachments(&attachment_descs)
            .subpasses(&subpass_descs)
            .build();

        let render_pass = unsafe { self.device_ctx.logical_info.device.create_render_pass(&render_pass_info, None)? };
        
        log::debug!("RenderPassCtx created");
        Ok(RenderPassCtx {
            swapchain_ctx: self,
            render_pass
        })
    }
}

impl Drop for RenderPassCtx<'_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_ctx.device_ctx.logical_info.device.destroy_render_pass(self.render_pass, None);
        }
        log::debug!("RenderPassCtx dropped");
    }
}