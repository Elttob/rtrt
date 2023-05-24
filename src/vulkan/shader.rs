use std::rc::Rc;

use anyhow::Result;
use ash::{vk::{ShaderModule, ShaderModuleCreateInfo}};

use super::device::DeviceCtx;

pub struct ShaderCtx {
    pub device_ctx: Rc<DeviceCtx>,
    pub module: ShaderModule,
    pub debug_name: String
}

impl ShaderCtx {
    pub fn new(
        device_ctx: Rc<DeviceCtx>,
        spirv: &[u32],
        debug_name: String
    ) -> Result<Rc<ShaderCtx>> {
        let create_info = ShaderModuleCreateInfo::builder()
            .code(spirv);
        let module = unsafe { device_ctx.logical_info.device.create_shader_module(&create_info, None)? };

        log::debug!("ShaderCtx created ({})", debug_name);
        Ok(Rc::new(ShaderCtx {
            device_ctx,
            module,
            debug_name
        }))
    }
}

impl Drop for ShaderCtx {
    fn drop(&mut self) {
        unsafe {
            self.device_ctx.logical_info.device.destroy_shader_module(self.module, None);
        }
        log::debug!("ShaderCtx dropped ({})", self.debug_name);
    }
}