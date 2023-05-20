use std::ffi::CString;

use anyhow::Result;
use ash::vk;

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