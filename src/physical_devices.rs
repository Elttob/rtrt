use std::ffi::CStr;

use anyhow::Result;
use ash::vk;

pub fn select(
    instance: &ash::Instance
) -> Result<vk::PhysicalDevice> {
    let devices = unsafe { instance.enumerate_physical_devices()? };
    let device = devices.into_iter()
        .find(|device| is_device_suitable(instance, *device))
        .ok_or(anyhow::anyhow!("No suitable physical device"))?;

    let props = unsafe { instance.get_physical_device_properties(device) };
    log::debug!("Selected physical device: {:?}", unsafe {
        CStr::from_ptr(props.device_name.as_ptr())
    });
    Ok(device)
}

fn is_device_suitable(instance: &ash::Instance, device: vk::PhysicalDevice) -> bool {
    find_queue_families(instance, device).is_some()
}

fn find_queue_families(instance: &ash::Instance, device: vk::PhysicalDevice) -> Option<usize> {
    let props = unsafe { instance.get_physical_device_queue_family_properties(device) };
    props.iter().enumerate()
        .find(|(_, family)| {
            family.queue_count > 0 && family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
        })
        .map(|(index, _)| index)
}