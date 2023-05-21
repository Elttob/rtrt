use std::{ffi::CStr, sync::Arc};

use anyhow::Result;
use ash::Entry;

use winit::window::Window;

use crate::ctx::{debug, instance::InstanceCtx};

use super::{surface::SurfaceCtx, debug::{DebugCtx, MessageSeverityFlags, MessageTypeFlags}, instance::AppInfo, device::DeviceCtx};
pub struct OverallCtx {
    pub entry: Arc<Entry>,
    pub instance_ctx: Arc<InstanceCtx>,
    pub debug_ctx: Option<Arc<DebugCtx>>,
    pub surface_ctx: Arc<SurfaceCtx>,
    pub device_ctx: Arc<DeviceCtx>
}

impl OverallCtx {
    pub fn new(
        window: Arc<Window>,
        app_info: AppInfo,
        user_extensions: &[&CStr],
        validation: Option<(MessageSeverityFlags, MessageTypeFlags)>
    ) -> Result<Self> {
        let entry = Arc::new(Entry::linked());
        let instance_ctx = Arc::new(InstanceCtx::new(entry.clone(), app_info, user_extensions, validation)?);
        let debug_ctx = if let Some((message_severity, message_type)) = validation {
            Some(Arc::new(debug::DebugCtx::new(instance_ctx.clone(), message_severity, message_type)?))
        } else {
            None
        };
        let surface_ctx = Arc::new(SurfaceCtx::new(instance_ctx.clone(), window.clone())?);
        let device_ctx = Arc::new(DeviceCtx::new(surface_ctx.clone())?);
        
        log::debug!("OverallCtx created");
        Ok(Self {
            entry,
            instance_ctx,
            debug_ctx,
            surface_ctx,
            device_ctx
        })
    }
}

impl Drop for OverallCtx {
    fn drop(&mut self) {
        log::debug!("OverallCtx dropped");
    }
}