use std::ffi::{CStr, CString, c_char};

use anyhow::Result;
use ash::{vk, extensions::ext::DebugUtils, Instance, Entry, Device};

mod validation;
pub mod app_info;

pub use validation::{MessageSeverityFlags, MessageTypeFlags};

use self::app_info::AppInfo;

pub struct DeviceCtx {
    pub instance: ash::Instance,
    #[allow(dead_code)]
    debug_utils_messenger: Option<validation::DebugUtilsMessenger>,
    pub device: ash::Device
}

const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];

impl DeviceCtx {
    pub fn new(
        entry: &Entry,
        app_info: AppInfo,
        enabled_extension_names: &[&CStr],
        validation: Option<(MessageSeverityFlags, MessageTypeFlags)>
    ) -> Result<Self> {
        log::debug!("DeviceCtx creating");
        let (_layer_names, layer_name_pointers) = Self::get_layer_names_and_pointers(validation.is_some())?;
        let app_info = app_info.try_into()?;
        let mut enabled_extension_names = enabled_extension_names.iter().map(|x| x.as_ptr()).collect::<Vec<_>>();
        if validation.is_some() {
            enabled_extension_names.push(DebugUtils::name().as_ptr());
        }
        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_extension_names)
            .enabled_layer_names(&layer_name_pointers);
        let instance = unsafe { entry.create_instance(&instance_create_info, None) }?;
        let debug_utils_messenger = if let Some((message_severity, message_type)) = validation {
            Some(validation::DebugUtilsMessenger::new(entry, &instance, message_severity, message_type)?)
        } else {
            None
        };
        let physical_device = Self::select_physical_device(&instance)?;
        let (device, _graphics_queue) = Self::create_logical_device_with_graphics_queue(&instance, physical_device, &layer_name_pointers)?;
        
        Ok(Self {
            instance,
            debug_utils_messenger,
            device
        })
    }

    fn get_layer_names_and_pointers(
        with_validation: bool
    ) -> Result<(Vec<CString>, Vec<*const c_char>)> {
        if with_validation {
            let names = VALIDATION_LAYERS.iter().cloned()
                .map(CString::new)
                .collect::<Result<Vec<_>, _>>()?;
            let pointers = names.iter()
                .map(|name| name.as_ptr())
                .collect::<Vec<_>>();
            Ok((names, pointers))
        } else {
            Ok((vec![], vec![]))
        }
    }

    fn select_physical_device(
        instance: &Instance,
    ) -> Result<vk::PhysicalDevice> {
        let devices = unsafe { instance.enumerate_physical_devices() }?;
        let device = devices.into_iter()
            .find(|device| Self::is_device_suitable(instance, *device))
            .ok_or(anyhow::anyhow!("No suitable physical device"))?;
        let props = unsafe { ash::Instance::get_physical_device_properties(instance.into(), device) };
        log::debug!("Selected physical device: {:?}", unsafe {
            CStr::from_ptr(props.device_name.as_ptr())
        });
        Ok(device)
    }
    
    fn is_device_suitable(instance: &Instance, device: vk::PhysicalDevice) -> bool {
        Self::find_queue_families(instance, device).is_some()
    }
    
    fn find_queue_families(instance: &Instance, device: vk::PhysicalDevice) -> Option<u32> {
        let props = unsafe { instance.get_physical_device_queue_family_properties(device) };
        props.iter().enumerate()
            .find(|(_, family)| {
                family.queue_count > 0 && family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            })
            .map(|(index, _)| index as u32)
    }
    
    fn create_logical_device_with_graphics_queue(
        instance: &Instance,
        device: vk::PhysicalDevice,
        layer_name_pointers: &[*const c_char]
    ) -> Result<(Device, vk::Queue)> {
        let queue_family_index = Self::find_queue_families(instance, device).ok_or(anyhow::anyhow!("No queue families found"))?;
        let queue_priorities = [1.0f32];
        let queue_create_infos = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities)
            .build()];
        let device_features = vk::PhysicalDeviceFeatures::builder().build();
        let device_create_info_builder = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&device_features)
            .enabled_layer_names(layer_name_pointers);
        let device_create_info = device_create_info_builder.build();
        let device = unsafe { instance.create_device(device, &device_create_info, None)? };
        let graphics_queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        log::debug!("Created logical device and graphics queue.");
        Ok((device, graphics_queue))
    }
}

impl Drop for DeviceCtx {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            if let Some(messenger) = &self.debug_utils_messenger {
                messenger.destroy();
            }
            self.instance.destroy_instance(None);
        }
        log::debug!("DeviceCtx dropped");
    }
}