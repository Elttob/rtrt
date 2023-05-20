use std::ffi::{c_void, CStr};

use anyhow::{Result, bail};
use ash::{extensions::ext::DebugUtils, vk::{DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT, Bool32, DebugUtilsMessengerCallbackDataEXT, self, DebugUtilsMessengerEXT}, Entry, Instance};

#[derive(Debug, strum_macros::Display)]
pub enum MessageSeverity {
    Info,
    Warning,
    Error,
    Verbose
}

impl From<MessageSeverity> for DebugUtilsMessageSeverityFlagsEXT {
    fn from(value: MessageSeverity) -> Self {
        match value {
            MessageSeverity::Info => Self::INFO,
            MessageSeverity::Warning => Self::WARNING,
            MessageSeverity::Error => Self::ERROR,
            MessageSeverity::Verbose => Self::VERBOSE
        }
    }
}

impl TryFrom<DebugUtilsMessageSeverityFlagsEXT> for MessageSeverity {
    type Error = anyhow::Error;

    fn try_from(value: DebugUtilsMessageSeverityFlagsEXT) -> Result<Self> {
        match value {
            DebugUtilsMessageSeverityFlagsEXT::INFO => Ok(Self::Info),
            DebugUtilsMessageSeverityFlagsEXT::WARNING => Ok(Self::Warning),
            DebugUtilsMessageSeverityFlagsEXT::ERROR => Ok(Self::Error),
            DebugUtilsMessageSeverityFlagsEXT::VERBOSE => Ok(Self::Verbose),
            _ => bail!("Not a valid single value")
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MessageSeverityFlags {
    pub info: bool,
    pub warning: bool,
    pub error: bool,
    pub verbose: bool
}

impl MessageSeverityFlags {
    pub fn all() -> Self {
        Self {
            info: true,
            warning: true,
            error: true,
            verbose: true
        }
    }
}

impl Default for MessageSeverityFlags {
    fn default() -> Self {
        Self {
            info: false,
            warning: false,
            error: false,
            verbose: false
        }
    }
}

impl From<MessageSeverityFlags> for DebugUtilsMessageSeverityFlagsEXT {
    fn from(value: MessageSeverityFlags) -> Self {
        let mut severity = Self::empty();
        if value.info {
            severity |= Self::INFO;
        }
        if value.warning {
            severity |= Self::WARNING;
        }
        if value.error {
            severity |= Self::ERROR;
        }
        if value.verbose {
            severity |= Self::VERBOSE;
        }
        severity
    }
}

#[derive(Debug, strum_macros::Display)]
pub enum MessageType {
    DeviceAddressBinding,
    General,
    Performance,
    Validation
}

impl From<MessageType> for DebugUtilsMessageTypeFlagsEXT {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::DeviceAddressBinding => Self::DEVICE_ADDRESS_BINDING,
            MessageType::General => Self::GENERAL,
            MessageType::Performance => Self::PERFORMANCE,
            MessageType::Validation => Self::VALIDATION
        }
    }
}

impl TryFrom<DebugUtilsMessageTypeFlagsEXT> for MessageType {
    type Error = anyhow::Error;

    fn try_from(value: DebugUtilsMessageTypeFlagsEXT) -> Result<Self> {
        match value {
            DebugUtilsMessageTypeFlagsEXT::DEVICE_ADDRESS_BINDING => Ok(Self::DeviceAddressBinding),
            DebugUtilsMessageTypeFlagsEXT::GENERAL => Ok(Self::General),
            DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => Ok(Self::Performance),
            DebugUtilsMessageTypeFlagsEXT::VALIDATION => Ok(Self::Validation),
            _ => bail!("Not a valid single value")
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MessageTypeFlags {
    pub device_address_binding: bool,
    pub general: bool,
    pub performance: bool,
    pub validation: bool
}

impl MessageTypeFlags {
    pub fn all() -> Self {
        Self {
            device_address_binding: true,
            general: true,
            performance: true,
            validation: true
        }
    }
}

impl Default for MessageTypeFlags {
    fn default() -> Self {
        Self {
            device_address_binding: false,
            general: false,
            performance: false,
            validation: false
        }
    }
}

impl From<MessageTypeFlags> for DebugUtilsMessageTypeFlagsEXT {
    fn from(value: MessageTypeFlags) -> Self {
        let mut severity = Self::empty();
        if value.device_address_binding {
            severity |= Self::DEVICE_ADDRESS_BINDING
        }
        if value.general {
            severity |= Self::GENERAL;
        }
        if value.performance {
            severity |= Self::PERFORMANCE;
        }
        if value.validation {
            severity |= Self::VALIDATION;
        }
        severity
    }
}
pub struct DebugUtilsMessenger {
    debug_utils: DebugUtils,
    messenger: DebugUtilsMessengerEXT
}

impl DebugUtilsMessenger {
    unsafe extern "system" fn vk_message_callback(
        message_severity: DebugUtilsMessageSeverityFlagsEXT,
        _message_types: DebugUtilsMessageTypeFlagsEXT,
        callback_data: *const DebugUtilsMessengerCallbackDataEXT,
        _user_data: *mut c_void,
    ) -> Bool32 {
        let severity_str = if let Ok(message_severity) = MessageSeverity::try_from(message_severity) {
            message_severity.to_string()
        } else {
            "(vkw: unknown)".to_string()
        };
        let message = if let Some(callback_data) = callback_data.as_ref() {
            CStr::from_ptr(callback_data.p_message).to_str().unwrap_or("(vkw: could not read p_message)")
        } else {
            "(vkw: could not read callback_data)"
        };
        log::debug!("[VK/{}] {}", severity_str, message); 
        vk::FALSE
    }

    pub fn new(
        entry: &Entry,
        instance: &Instance,
        message_severity: MessageSeverityFlags,
        message_type: MessageTypeFlags
    ) -> Result<Self> {
        log::debug!("DebugUtilsMessenger creating");
        let debug_utils = DebugUtils::new(entry, instance);
        let create_info = DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(message_severity.into())
            .message_type(message_type.into())
            .pfn_user_callback(Some(Self::vk_message_callback));
        let messenger = unsafe { debug_utils.create_debug_utils_messenger(&create_info, None) }?;
        
        Ok(Self {
            debug_utils,
            messenger
        })
    }

    pub unsafe fn destroy(&self) {
        self.debug_utils.destroy_debug_utils_messenger(self.messenger, None);
        log::debug!("DebugUtilsMessenger dropped");
    }
}