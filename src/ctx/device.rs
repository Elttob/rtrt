use std::{sync::Arc, ffi::{CStr, c_char, CString}};
use anyhow::Result;
use ash::{vk::{self, PhysicalDevice, Queue}, Device};

use super::{instance::InstanceCtx, surface::SurfaceCtx};
pub struct DeviceCtx {
    pub surface_ctx: Arc<SurfaceCtx>,
    pub physical_device: PhysicalDevice,
    pub device: Device,
    pub graphics_queue: Queue,
    pub present_queue: Queue
}

impl DeviceCtx {
    pub fn new(
        surface_ctx: Arc<SurfaceCtx>
    ) -> Result<Self> {
        let instance_ctx = &surface_ctx.instance_ctx;
        let (physical_device, debug_device_name) = Self::select_physical_device(&instance_ctx, &surface_ctx)?;
        let (device, graphics_queue, present_queue) = Self::create_logical_device(&instance_ctx, &surface_ctx, physical_device, &instance_ctx.layer_name_pointers)?;
        
        log::debug!("DeviceCtx created ({})", debug_device_name.to_str().unwrap_or("device is not nameable"));
        Ok(Self {
            surface_ctx,
            physical_device,
            device,
            graphics_queue,
            present_queue,
        })
    }

    fn select_physical_device(
        instance_ctx: &InstanceCtx,
        surface_ctx: &SurfaceCtx
    ) -> Result<(vk::PhysicalDevice, CString)> {
        let devices = unsafe { instance_ctx.instance.enumerate_physical_devices() }?;
        let device = devices.into_iter()
            .find(|device| Self::is_device_suitable(instance_ctx, surface_ctx, *device))
            .ok_or(anyhow::anyhow!("No suitable physical device"))?;
        let props = unsafe { ash::Instance::get_physical_device_properties(&instance_ctx.instance, device) };
        let debug_device_name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }.to_owned();
        Ok((device, debug_device_name))
    }
    
    fn is_device_suitable(
        instance_ctx: &InstanceCtx,
        surface_ctx: &SurfaceCtx,
        device: vk::PhysicalDevice,
    ) -> bool {
        Self::find_queue_families(instance_ctx, surface_ctx, device).is_some()
    }
    
    fn find_queue_families(
        instance_ctx: &InstanceCtx,
        surface_ctx: &SurfaceCtx,
        device: vk::PhysicalDevice
    ) -> Option<(u32, u32)> {
        let mut graphics = None;
        let mut present = None;
        let props = unsafe { instance_ctx.instance.get_physical_device_queue_family_properties(device) };
        for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
            let index = index as u32;
            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics.is_none() {
                graphics = Some(index);
            }
            let present_support = unsafe { surface_ctx.surface.get_physical_device_surface_support(device, index, surface_ctx.surface_khr).unwrap_or(false) };
            if present_support && present.is_none() {
                present = Some(index);
            }
            if let Some(graphics) = graphics {
                if let Some(present) = present {
                    return Some((graphics, present))
                }
            }
        }
        None
    }
    
    fn create_logical_device(
        instance_ctx: &InstanceCtx,
        surface_ctx: &SurfaceCtx,
        physical_device: vk::PhysicalDevice,
        layer_name_pointers: &[*const c_char]
    ) -> Result<(Device, vk::Queue, vk::Queue)> {
        let (graphics_family_index, present_family_index) = Self::find_queue_families(instance_ctx, surface_ctx, physical_device).ok_or(anyhow::anyhow!("No queue families found"))?;
        let queue_priorities = [1.0f32];
        let queue_create_infos = {
            let mut indices = vec![graphics_family_index, present_family_index];
            indices.dedup();
            indices.iter()
            .map(|index| vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*index)
                    .queue_priorities(&queue_priorities)
                    .build()
            ).collect::<Vec<_>>()
        };
        let device_features = vk::PhysicalDeviceFeatures::builder().build();
        let device_create_info_builder = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&device_features)
            .enabled_layer_names(layer_name_pointers);
        let device_create_info = device_create_info_builder.build();
        let device = unsafe { instance_ctx.instance.create_device(physical_device, &device_create_info, None)? };
        let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family_index, 0) };
        Ok((device, graphics_queue, present_queue))
    }
}

impl Drop for DeviceCtx {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
        log::debug!("DeviceCtx dropped");
    }
}