use anyhow::Result;
use ash::{vk::{ShaderModule, ShaderModuleCreateInfo}};

use super::device::DeviceCtx;

pub struct ShaderCtx<'dev, 'srf, 'ins, 'en> {
    pub device_ctx: &'dev DeviceCtx<'srf, 'ins, 'en>,
    pub module: ShaderModule
}

impl<'srf, 'ins, 'en> DeviceCtx<'srf, 'ins, 'en> {
    pub fn create_shader_ctx(
        &self,
        spirv: &[u32]
    ) -> Result<ShaderCtx> {
        let create_info = ShaderModuleCreateInfo::builder()
            .code(spirv);
        let module = unsafe { self.logical_info.device.create_shader_module(&create_info, None)? };
        
        log::debug!("ShaderCtx created");
        Ok(ShaderCtx {
            device_ctx: self,
            module
        })
    }
}

impl Drop for ShaderCtx<'_, '_, '_, '_> {
    fn drop(&mut self) {
        unsafe {
            self.device_ctx.logical_info.device.destroy_shader_module(self.module, None);
        }
        log::debug!("ShaderCtx dropped");
    }
}