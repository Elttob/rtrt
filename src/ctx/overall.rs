use std::ffi::{CStr, c_char};

use anyhow::Result;
use ash::{Instance, Device, vk};

use winit::window::Window;

use crate::ctx::{debug, instance::InstanceCtx};

use super::{surface::SurfaceCtx, debug::{DebugCtx, MessageSeverityFlags, MessageTypeFlags}, app_info::AppInfo};
pub struct OverallCtx {
    pub instance_ctx: InstanceCtx,
    pub surface_ctx: SurfaceCtx,
    pub debug_ctx: Option<DebugCtx>,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue
}

impl OverallCtx {
    pub fn new(
        window: &Window,
        app_info: AppInfo,
        user_extensions: &[&CStr],
        validation: Option<(MessageSeverityFlags, MessageTypeFlags)>
    ) -> Result<Self> {
        log::debug!("DeviceCtx creating");
        let entry = ash::Entry::linked();
        let instance_ctx = InstanceCtx::new(&entry, app_info, user_extensions, validation)?;
        let debug_utils_messenger = if let Some((message_severity, message_type)) = validation {
            Some(debug::DebugCtx::new(&entry, &instance, message_severity, message_type)?)
        } else {
            None
        };

        let surface_ctx = SurfaceCtx::new(&entry, &instance, window)?;
        let physical_device = Self::select_physical_device(&instance, &surface_ctx)?;
        let (device, graphics_queue, present_queue) = Self::create_logical_device(&instance, &surface_ctx, physical_device, &layer_name_pointers)?;
        
        Ok(Self {
            instance,
            surface_ctx,
            debug_ctx: debug_utils_messenger,
            device,
            graphics_queue,
            present_queue
        })
    }

    fn select_physical_device(
        instance: &Instance,
        surface_ctx: &SurfaceCtx
    ) -> Result<vk::PhysicalDevice> {
        let devices = unsafe { instance.enumerate_physical_devices() }?;
        let device = devices.into_iter()
            .find(|device| Self::is_device_suitable(instance, surface_ctx, *device))
            .ok_or(anyhow::anyhow!("No suitable physical device"))?;
        let props = unsafe { ash::Instance::get_physical_device_properties(instance.into(), device) };
        log::debug!("Selected physical device: {:?}", unsafe {
            CStr::from_ptr(props.device_name.as_ptr())
        });
        Ok(device)
    }
    
    fn is_device_suitable(
        instance: &Instance,
        surface_ctx: &SurfaceCtx,
        device: vk::PhysicalDevice,
    ) -> bool {
        Self::find_queue_families(instance, surface_ctx, device).is_some()
    }
    
    fn find_queue_families(
        instance: &Instance, 
        surface_ctx: &SurfaceCtx,
        device: vk::PhysicalDevice
    ) -> Option<(u32, u32)> {
        let mut graphics = None;
        let mut present = None;
        let props = unsafe { instance.get_physical_device_queue_family_properties(device) };
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
        instance: &Instance,
        surface_ctx: &SurfaceCtx,
        physical_device: vk::PhysicalDevice,
        layer_name_pointers: &[*const c_char]
    ) -> Result<(Device, vk::Queue, vk::Queue)> {
        let (graphics_family_index, present_family_index) = Self::find_queue_families(instance, surface_ctx, physical_device).ok_or(anyhow::anyhow!("No queue families found"))?;
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
        let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };
        let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family_index, 0) };
        log::debug!("Created logical device w/ graphics queue {} & present queue {}.", graphics_family_index, present_family_index);
        Ok((device, graphics_queue, present_queue))
    }
}

impl Drop for OverallCtx {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            if let Some(messenger) = &self.debug_ctx {
                messenger.destroy();
            }
            
        }
        log::debug!("DeviceCtx dropped");
    }
}