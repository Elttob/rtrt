use std::{ffi::{CStr, c_char, CString}};
use anyhow::Result;
use ash::{vk::{self, PhysicalDevice, Queue}, Device, extensions::khr::Swapchain};

use super::surface::{SurfaceCtx, SwapchainSupportDetails};

pub const REQUIRED_DEVICE_EXT: &[&CStr] = &[Swapchain::name()];
pub struct DeviceCtx<'srf, 'ins, 'en> {
    pub surface_ctx: &'srf SurfaceCtx<'ins, 'en>,
    pub physical_device: SelectedPhysicalDevice,
    pub logical_device: SelectedLogicalDevice
}

impl<'ins, 'en> SurfaceCtx<'ins, 'en> {
    pub fn create_device_ctx(
        &self
    ) -> Result<DeviceCtx> {
        let instance_ctx = &self.instance_ctx;
        let physical_device = self.select_physical_device()?;
        let logical_device = self.create_logical_device(&physical_device, &instance_ctx.layer_name_pointers)?;
        
        log::debug!("DeviceCtx created ({})", physical_device.debug_device_name.to_str().unwrap_or("vkw: device is not nameable"));
        Ok(DeviceCtx {
            surface_ctx: self,
            physical_device,
            logical_device
        })
    }

    fn select_physical_device(
        &self
    ) -> Result<SelectedPhysicalDevice> {
        let devices = unsafe { self.instance_ctx.instance.enumerate_physical_devices() }?;
        let devices_and_queues = devices.into_iter()
            .map(|device| Ok((device, self.find_queue_families(device)?)))
            .collect::<Result<Vec<_>>>()?;
        devices_and_queues.into_iter()
        .filter_map(|(device, queues)| {
            let (graphics_family_index, present_family_index) = queues?;
            let supports_required_extensions = self.test_required_extensions(device).ok()?;
            if !supports_required_extensions { return None; }
            let swapchain_support_details = self.swapchain_support_details(device).ok()?;
            let swapchain_is_adequate = !swapchain_support_details.formats.is_empty() && !swapchain_support_details.present_modes.is_empty();
            if !swapchain_is_adequate { return None; }
            let props = unsafe { self.instance_ctx.instance.get_physical_device_properties(device) };
            let debug_device_name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }.to_owned();
            let dedup_family_indices = if graphics_family_index == present_family_index { vec![graphics_family_index] } else { vec![graphics_family_index, present_family_index] };
            Some(SelectedPhysicalDevice {
                device,
                graphics_family_index,
                present_family_index,
                dedup_family_indices,
                swapchain_support_details,
                debug_device_name,
            })
        })
        .next().ok_or(anyhow::anyhow!("No suitable physical device"))
    }

    fn test_required_extensions(
        &self,
        device: PhysicalDevice
    ) -> Result<bool> {
        let extension_props = unsafe { self.instance_ctx.instance.enumerate_device_extension_properties(device)? };
        let extension_names = extension_props.iter()
            .map(|x| unsafe { CStr::from_ptr(x.extension_name.as_ptr()) })
            .collect::<Vec<_>>();
        let has_all_extensions = REQUIRED_DEVICE_EXT.iter().all(|x| extension_names.contains(x));
        Ok(has_all_extensions)
    }
    
    fn find_queue_families(
        &self,
        device: vk::PhysicalDevice
    ) -> Result<Option<(u32, u32)>> {
        let mut graphics = None;
        let mut present = None;
        let props = unsafe { self.instance_ctx.instance.get_physical_device_queue_family_properties(device) };
        for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
            let index = index as u32;
            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics.is_none() {
                graphics = Some(index);
            }
            let present_support = unsafe { self.surface.get_physical_device_surface_support(device, index, self.surface_khr)? };
            if present_support && present.is_none() {
                present = Some(index);
            }
            if let Some(graphics) = graphics {
                if let Some(present) = present {
                    return Ok(Some((graphics, present)))
                }
            }
        }
        Ok(None)
    }
    
    fn create_logical_device(
        &self,
        physical_device: &SelectedPhysicalDevice,
        layer_name_pointers: &[*const c_char]
    ) -> Result<SelectedLogicalDevice> {
        let queue_priorities = [1.0f32];
        let queue_create_infos = physical_device.dedup_family_indices.iter()
            .map(|index| vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(*index)
                .queue_priorities(&queue_priorities)
                .build()
            ).collect::<Vec<_>>();
        let device_extensions_ptrs = REQUIRED_DEVICE_EXT.iter().map(|x| x.as_ptr()).collect::<Vec<_>>();
        let device_features = vk::PhysicalDeviceFeatures::builder().build();
        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions_ptrs)
            .enabled_features(&device_features)
            .enabled_layer_names(layer_name_pointers)
            .build();
        let device = unsafe { self.instance_ctx.instance.create_device(physical_device.device, &device_create_info, None)? };
        let graphics_queue = unsafe { device.get_device_queue(physical_device.graphics_family_index, 0) };
        let present_queue = unsafe { device.get_device_queue(physical_device.present_family_index, 0) };
        Ok(SelectedLogicalDevice {
            device,
            graphics_queue,
            present_queue
        })
    }
}

impl Drop for DeviceCtx<'_, '_, '_> {
    fn drop(&mut self) {
        unsafe {
            self.logical_device.device.destroy_device(None);
        }
        log::debug!("DeviceCtx dropped");
    }
}

pub struct SelectedPhysicalDevice {
    pub device: PhysicalDevice,
    pub graphics_family_index: u32,
    pub present_family_index: u32,
    pub dedup_family_indices: Vec<u32>,
    pub swapchain_support_details: SwapchainSupportDetails,
    pub debug_device_name: CString
}

pub struct SelectedLogicalDevice {
    pub device: Device,
    pub graphics_queue: Queue,
    pub present_queue: Queue
}