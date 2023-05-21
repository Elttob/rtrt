use std::{ffi::{CStr, CString, c_char}, sync::Arc};
use anyhow::Result;
use ash::{Instance, Entry, vk};

use crate::ctx::{surface, debug};

use super::{debug::{MessageSeverityFlags, MessageTypeFlags}};

pub struct InstanceCtx {
    pub entry: Arc<Entry>,
    pub instance: Instance,
    pub layer_names: Vec<CString>,
    pub layer_name_pointers: Vec<*const i8>
}

impl InstanceCtx {
    pub fn new(
        entry: Arc<Entry>,
        app_info: AppInfo,
        user_extensions: &[&CStr],
        validation: Option<(MessageSeverityFlags, MessageTypeFlags)>
    ) -> Result<Self> {
        let (layer_names, layer_name_pointers) = Self::get_layer_names_and_pointers(validation.is_some())?;
        let app_info = app_info.try_into()?;
        let all_extensions = user_extensions.into_iter()
            .map(|x| x.as_ptr())
            .chain(surface::required_extension_names_win32().into_iter())
            .chain(debug::required_extension_names(validation.is_some()).into_iter())
            .collect::<Vec<_>>();
        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&all_extensions)
            .enabled_layer_names(&layer_name_pointers);
        let instance = unsafe { entry.create_instance(&instance_create_info, None) }?;

        log::debug!("InstanceCtx created");
        Ok(Self {
            entry,
            instance,
            layer_names,
            layer_name_pointers
        })
    }

    fn get_layer_names_and_pointers(
        with_validation: bool
    ) -> Result<(Vec<CString>, Vec<*const c_char>)> {
        if with_validation {
            let names = debug::VALIDATION_LAYERS.iter().cloned()
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
}

impl Drop for InstanceCtx {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
        log::debug!("InstanceCtx dropped");
    }
}

// SUPPORTING TYPES

#[derive(Debug, Clone, Copy)]
pub struct Version {
    variant: u32,
    major: u32,
    minor: u32,
    patch: u32
}

impl From<Version> for u32 {
    fn from(value: Version) -> Self {
        vk::make_api_version(value.variant, value.major, value.minor, value.patch)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self {
            variant: 0,
            major: 0,
            minor: 1,
            patch: 0
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AppInfo<'a> {
    application_name: &'a str,
    application_version: Version,
    engine_name: &'a str,
    engine_version: Version,
    api_version: Version
}

impl<'a> Default for AppInfo<'a> {
    fn default() -> Self {
        Self {
            application_name: "Rust Vulkan App",
            application_version: Default::default(),
            engine_name: "No Engine",
            engine_version: Default::default(),
            api_version: Default::default()
        }
    }
}

impl<'a> TryFrom<AppInfo<'a>> for vk::ApplicationInfo {
    type Error = anyhow::Error;

    fn try_from(value: AppInfo<'a>) -> Result<Self> {
        Ok(
            vk::ApplicationInfo::builder()
            .application_name(CString::new(value.application_name)?.as_c_str())
            .application_version(value.application_version.into())
            .engine_name(CString::new(value.engine_name)?.as_c_str())
            .engine_version(value.engine_version.into())
            .api_version(value.api_version.into())
            .build()
        )
    }
}