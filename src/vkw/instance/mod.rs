use std::{marker::PhantomData, ffi::{CStr, CString}};

use anyhow::Result;
use ash::{vk, extensions::ext::DebugUtils};

use super::app_info::AppInfo;

mod validation;

pub use validation::{MessageSeverityFlags, MessageTypeFlags};

pub struct Instance<'entry> {
    instance: ash::Instance,
    #[allow(dead_code)]
    debug_utils_messenger: Option<validation::DebugUtilsMessenger>,
    _entry: PhantomData<&'entry ash::Entry>
}

const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];

impl<'entry> Instance<'entry> {
    pub fn new(
        entry: &'entry ash::Entry,
        app_info: AppInfo,
        enabled_extension_names: &[&CStr],
        enable_validation: Option<(MessageSeverityFlags, MessageTypeFlags)>
    ) -> Result<Self> {
        log::debug!("Instance creating");
        let validation_layers_c = VALIDATION_LAYERS.iter().cloned()
            .map(CString::new)
            .collect::<Result<Vec<_>, _>>()?;
        let validation_layers_c_ptr = validation_layers_c.iter()
            .map(|name| name.as_ptr())
            .collect::<Vec<_>>();

        let app_info = app_info.try_into()?;
        let mut enabled_extension_names = enabled_extension_names.iter().map(|x| x.as_ptr()).collect::<Vec<_>>();
        if enable_validation.is_some() {
            enabled_extension_names.push(DebugUtils::name().as_ptr());
        }
        let mut instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_extension_names);
        if enable_validation.is_some() {
            instance_create_info = instance_create_info.enabled_layer_names(&validation_layers_c_ptr)
        }
        let instance = unsafe { entry.create_instance(&instance_create_info, None) }?;

        let debug_utils_messenger = if let Some((message_severity, message_type)) = enable_validation {
            Some(validation::DebugUtilsMessenger::new(entry, &instance, message_severity, message_type)?)
        } else {
            None
        };
        
        Ok(Self {
            instance,
            debug_utils_messenger,
            _entry: PhantomData
        })
    }
}

impl<'a> From<&'a Instance<'_>> for &'a ash::Instance {
    fn from(value: &'a Instance) -> Self {
        &value.instance
    }
}

impl<'entry> Drop for Instance<'entry> {
    fn drop(&mut self) {
        if let Some(messenger) = &self.debug_utils_messenger {
            unsafe { messenger.destroy() }
        }
        unsafe { self.instance.destroy_instance(None); }
        log::debug!("Instance dropped");
    }
}