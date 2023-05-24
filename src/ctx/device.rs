use std::{ffi::{CStr, c_char, CString}, rc::Rc};
use anyhow::Result;
use ash::{vk::{self, PhysicalDevice, Queue}, Device, extensions::khr::Swapchain};

use super::surface::{SurfaceCtx, SwapchainSupportDetails};

pub const REQUIRED_DEVICE_EXT: &[&CStr] = &[Swapchain::name()];

fn select_physical_device(
    surface_ctx: &SurfaceCtx
) -> Result<PhysicalDeviceInfo> {
    let devices = unsafe { surface_ctx.instance_ctx.instance.enumerate_physical_devices() }?;
    let devices_and_queues = devices.into_iter()
        .map(|device| Ok((device, find_queue_families(surface_ctx, device)?)))
        .collect::<Result<Vec<_>>>()?;
    devices_and_queues.into_iter()
    .filter_map(|(device, queues)| {
        let (graphics_family_index, present_family_index) = queues?;
        let supports_required_extensions = test_required_extensions(surface_ctx, device).ok()?;
        if !supports_required_extensions { return None; }
        let swapchain_support_details = surface_ctx.swapchain_support_details(device).ok()?;
        let swapchain_is_adequate = !swapchain_support_details.formats.is_empty() && !swapchain_support_details.present_modes.is_empty();
        if !swapchain_is_adequate { return None; }
        let props = unsafe { surface_ctx.instance_ctx.instance.get_physical_device_properties(device) };
        let debug_device_name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }.to_owned();
        let dedup_family_indices = if graphics_family_index == present_family_index { vec![graphics_family_index] } else { vec![graphics_family_index, present_family_index] };
        Some(PhysicalDeviceInfo {
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
    surface_ctx: &SurfaceCtx,
    device: PhysicalDevice
) -> Result<bool> {
    let extension_props = unsafe { surface_ctx.instance_ctx.instance.enumerate_device_extension_properties(device)? };
    let extension_names = extension_props.iter()
        .map(|x| unsafe { CStr::from_ptr(x.extension_name.as_ptr()) })
        .collect::<Vec<_>>();
    let has_all_extensions = REQUIRED_DEVICE_EXT.iter().all(|x| extension_names.contains(x));
    Ok(has_all_extensions)
}

fn find_queue_families(
    surface_ctx: &SurfaceCtx,
    device: PhysicalDevice
) -> Result<Option<(u32, u32)>> {
    let mut graphics = None;
    let mut present = None;
    let props = unsafe { surface_ctx.instance_ctx.instance.get_physical_device_queue_family_properties(device) };
    for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
        let index = index as u32;
        if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics.is_none() {
            graphics = Some(index);
        }
        let present_support = unsafe { surface_ctx.surface.get_physical_device_surface_support(device, index, surface_ctx.surface_khr)? };
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
    surface_ctx: &SurfaceCtx,
    physical_info: &PhysicalDeviceInfo,
    layer_name_pointers: &[*const c_char]
) -> Result<LogicalDeviceInfo> {
    let queue_priorities = [1.0f32];
    let queue_create_infos = physical_info.dedup_family_indices.iter()
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
    let device = unsafe { surface_ctx.instance_ctx.instance.create_device(physical_info.device, &device_create_info, None)? };
    let graphics_queue = unsafe { device.get_device_queue(physical_info.graphics_family_index, 0) };
    let present_queue = unsafe { device.get_device_queue(physical_info.present_family_index, 0) };
    Ok(LogicalDeviceInfo {
        device,
        graphics_queue,
        present_queue
    })
}
pub struct DeviceCtx {
    pub surface_ctx: Rc<SurfaceCtx>,
    pub physical_info: PhysicalDeviceInfo,
    pub logical_info: LogicalDeviceInfo
}

impl DeviceCtx {
    pub fn new(
        surface_ctx: Rc<SurfaceCtx>
    ) -> Result<Rc<DeviceCtx>> {
        let physical_info = select_physical_device(&surface_ctx)?;
        let logical_info = create_logical_device(&surface_ctx, &physical_info, &surface_ctx.instance_ctx.layer_name_pointers)?;
        
        log::debug!("DeviceCtx created ({})", physical_info.debug_device_name.to_str().unwrap_or("vkw: device is not nameable"));
        Ok(Rc::new(DeviceCtx {
            surface_ctx,
            physical_info,
            logical_info
        }))
    }
}

impl Drop for DeviceCtx {
    fn drop(&mut self) {
        unsafe {
            self.logical_info.device.destroy_device(None);
        }
        log::debug!("DeviceCtx dropped");
    }
}

pub struct PhysicalDeviceInfo {
    pub device: PhysicalDevice,
    pub graphics_family_index: u32,
    pub present_family_index: u32,
    pub dedup_family_indices: Vec<u32>,
    pub swapchain_support_details: SwapchainSupportDetails,
    pub debug_device_name: CString
}

pub struct LogicalDeviceInfo {
    pub device: Device,
    pub graphics_queue: Queue,
    pub present_queue: Queue
}