use std::rc::Rc;

use ash::vk::{Fence, FenceCreateInfo, FenceCreateFlags};
use anyhow::Result;
use super::device::DeviceCtx;

pub struct FenceCtx {
    pub device_ctx: Rc<DeviceCtx>,
    pub fence: Fence
}

impl FenceCtx {
    pub fn new(
        device_ctx: Rc<DeviceCtx>,
        flags: FenceCreateFlags
    ) -> Result<Rc<FenceCtx>> {
        let create_info = FenceCreateInfo::builder()
            .flags(flags)
            .build();
        let fence = unsafe { device_ctx.logical_info.device.create_fence(&create_info, None)? };

        log::debug!("FenceCtx created");
        Ok(Rc::new(FenceCtx {
            device_ctx,
            fence
        }))
    }
}

impl Drop for FenceCtx {
    fn drop(&mut self) {
        unsafe {
            self.device_ctx.logical_info.device.destroy_fence(self.fence, None);
        }
        log::debug!("FenceCtx dropped");
    }
}