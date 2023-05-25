use std::rc::Rc;

use anyhow::Result;
use ash::vk::{AttachmentDescription, SampleCountFlags, AttachmentLoadOp, ImageLayout, AttachmentReference, SubpassDescription, PipelineBindPoint, AttachmentStoreOp, RenderPassCreateInfo, RenderPass, SubpassDependency, self, AccessFlags, PipelineStageFlags};

use super::swapchain::SwapchainCtx;

pub struct RenderPassCtx {
    pub swapchain_ctx: Rc<SwapchainCtx>,
    pub render_pass: RenderPass
}

impl RenderPassCtx {
    pub fn new(
        swapchain_ctx: Rc<SwapchainCtx>
    ) -> Result<Rc<RenderPassCtx>> {
        let attachment_desc = AttachmentDescription::builder()
            .format(swapchain_ctx.swapchain_image_format)
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
        let subpass_dep = SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(AccessFlags::empty())
            .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(AccessFlags::COLOR_ATTACHMENT_READ | AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build();
        let subpass_deps = [subpass_dep];
        let render_pass_info = RenderPassCreateInfo::builder()
            .attachments(&attachment_descs)
            .subpasses(&subpass_descs)
            .dependencies(&subpass_deps)
            .build();

        let render_pass = unsafe { swapchain_ctx.device_ctx.logical_info.device.create_render_pass(&render_pass_info, None)? };
        
        log::debug!("RenderPassCtx created");
        Ok(Rc::new(RenderPassCtx {
            swapchain_ctx,
            render_pass
        }))
    }
}

impl Drop for RenderPassCtx {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_ctx.device_ctx.logical_info.device.destroy_render_pass(self.render_pass, None);
        }
        log::debug!("RenderPassCtx dropped");
    }
}