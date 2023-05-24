use std::rc::Rc;

use ash::vk::{Semaphore, SemaphoreCreateInfo};
use anyhow::Result;
use super::device::DeviceCtx;

pub struct SemaphoreCtx {
    pub device_ctx: Rc<DeviceCtx>,
    pub semaphore: Semaphore
}

impl SemaphoreCtx {
    pub fn new(
        device_ctx: Rc<DeviceCtx>
    ) -> Result<Rc<SemaphoreCtx>> {
        let create_info = SemaphoreCreateInfo::builder().build();
        let semaphore = unsafe { device_ctx.logical_info.device.create_semaphore(&create_info, None)? };

        log::debug!("SemaphoreCtx created");
        Ok(Rc::new(SemaphoreCtx {
            device_ctx,
            semaphore
        }))
    }
}

impl Drop for SemaphoreCtx {
    fn drop(&mut self) {
        unsafe {
            self.device_ctx.logical_info.device.destroy_semaphore(self.semaphore, None);
        }
        log::debug!("SemaphoreCtx dropped");
    }
}