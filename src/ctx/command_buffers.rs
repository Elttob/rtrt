use std::rc::Rc;

use anyhow::Result;
use ash::vk::{CommandBuffer, CommandBufferLevel, CommandBufferAllocateInfo};

use super::command_pool::CommandPoolCtx;

pub struct CommandBuffersCtx {
    pub command_pool_ctx: Rc<CommandPoolCtx>,
    pub buffers: Vec<CommandBuffer>
}

impl CommandBuffersCtx {
    pub fn new(
        command_pool_ctx: Rc<CommandPoolCtx>,
        level: CommandBufferLevel,
        count: u32
    ) -> Result<Rc<CommandBuffersCtx>> {
        let allocate_info = CommandBufferAllocateInfo::builder()
            .command_pool(command_pool_ctx.command_pool)
            .level(level)
            .command_buffer_count(count)
            .build();

        let buffers = unsafe { command_pool_ctx.device_ctx.logical_info.device.allocate_command_buffers(&allocate_info)? };
        
        Ok(Rc::new(CommandBuffersCtx {
            command_pool_ctx,
            buffers
        }))
    }
}