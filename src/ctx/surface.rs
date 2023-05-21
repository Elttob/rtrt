use std::{ptr, ffi::c_void, sync::Arc};
use anyhow::Result;
use ash::{extensions::khr::{Win32Surface, Surface}, Entry, Instance, vk::{self, PhysicalDevice, SurfaceCapabilitiesKHR, SurfaceFormatKHR, PresentModeKHR}};
use winit::{window::Window, platform::windows::WindowExtWindows};
use winapi::{shared::windef::HWND, um::libloaderapi::GetModuleHandleW};

use super::instance::InstanceCtx;

pub fn required_extension_names_win32() -> Vec<*const i8> {
    vec![Surface::name().as_ptr(), Win32Surface::name().as_ptr()]
}

pub unsafe fn create_surface_win32(
    entry: &Entry,
    instance: &Instance,
    window: &Window,
) -> Result<vk::SurfaceKHR, vk::Result> {
    let hwnd = window.hwnd() as HWND;
    let hinstance = GetModuleHandleW(ptr::null());
    let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
        s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
        p_next: ptr::null(),
        flags: Default::default(),
        hinstance: hinstance as *const c_void,
        hwnd: hwnd as *const c_void,
    };
    let win32_surface_loader = Win32Surface::new(entry, instance);
    win32_surface_loader.create_win32_surface(&win32_create_info, None)
}

pub struct SurfaceCtx<'ins, 'en> {
    pub instance_ctx: &'ins InstanceCtx<'en>,
    pub surface: Surface,
    pub surface_khr: vk::SurfaceKHR
}

impl SurfaceCtx<'_, '_> {
    pub fn swapchain_support_details(
        &self,
        physical_device: PhysicalDevice
    ) -> Result<SwapchainSupportDetails> {
        let capabilities = unsafe { self.surface.get_physical_device_surface_capabilities(physical_device, self.surface_khr)? };
        let formats = unsafe { self.surface.get_physical_device_surface_formats(physical_device, self.surface_khr)? };
        let present_modes = unsafe { self.surface.get_physical_device_surface_present_modes(physical_device, self.surface_khr)? };
        Ok(SwapchainSupportDetails {
            capabilities,
            formats,
            present_modes,
        })
    }
}

impl<'en> InstanceCtx<'en> {
    pub fn create_surface_ctx(
        &self,
        window: Arc<Window>
    ) -> Result<SurfaceCtx> {
        let surface = Surface::new(&self.entry_ctx.entry, &self.instance);
        let surface_khr = unsafe { create_surface_win32(&self.entry_ctx.entry, &self.instance, &window)? };

        log::debug!("SurfaceCtx created");
        Ok(SurfaceCtx {
            instance_ctx: self,
            surface,
            surface_khr
        })
    }
}

impl Drop for SurfaceCtx<'_, '_> {
    fn drop(&mut self) {
        unsafe {
            self.surface.destroy_surface(self.surface_khr, None);
        }
        log::debug!("SurfaceCtx dropped");
    }
}

// SUPPORTING TYPES

pub struct SwapchainSupportDetails {
    pub capabilities: SurfaceCapabilitiesKHR,
    pub formats: Vec<SurfaceFormatKHR>,
    pub present_modes: Vec<PresentModeKHR>,
}