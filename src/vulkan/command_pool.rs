use std::rc::Rc;

use anyhow::Result;
use ash::vk::{CommandPool, CommandPoolCreateFlags, CommandPoolCreateInfo};

use super::device::DeviceCtx;

pub struct CommandPoolCtx {
    pub device_ctx: Rc<DeviceCtx>,
    pub command_pool: CommandPool
}

impl CommandPoolCtx {
    pub fn new(
        device_ctx: Rc<DeviceCtx>,
        flags: CommandPoolCreateFlags,
    ) -> Result<Rc<CommandPoolCtx>> {
        let command_pool_info = CommandPoolCreateInfo::builder()
            .queue_family_index(device_ctx.physical_info.graphics_family_index)
            .flags(flags)
            .build();

        let command_pool = unsafe { device_ctx.logical_info.device.create_command_pool(&command_pool_info, None)? };
        
        log::debug!("CommandPoolCtx created");
        Ok(Rc::new(CommandPoolCtx {
            device_ctx,
            command_pool
        }))
    }
}

impl Drop for CommandPoolCtx {
    fn drop(&mut self) {
        unsafe {
            self.device_ctx.logical_info.device.destroy_command_pool(self.command_pool, None);
        }
        log::debug!("CommandPoolCtx dropped");
    }
}