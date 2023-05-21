use std::{ptr, ffi::c_void, sync::Arc};
use anyhow::Result;
use ash::{extensions::khr::{Win32Surface, Surface}, Entry, Instance, vk};
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

pub struct SurfaceCtx {
    pub instance_ctx: Arc<InstanceCtx>,
    pub surface: Surface,
    pub surface_khr: vk::SurfaceKHR
}

impl SurfaceCtx {
    pub fn new(
        instance_ctx: Arc<InstanceCtx>,
        window: Arc<Window>
    ) -> Result<Self> {
        log::debug!("SurfaceCtx creating");
        let surface = Surface::new(&instance_ctx.entry, &instance_ctx.instance);
        let surface_khr = unsafe { create_surface_win32(&instance_ctx.entry, &instance_ctx.instance, &window)? };
        Ok(Self {
            instance_ctx,
            surface,
            surface_khr
        })
    }
}

impl Drop for SurfaceCtx {
    fn drop(&mut self) {
        unsafe {
            self.surface.destroy_surface(self.surface_khr, None);
        }
        log::debug!("SurfaceCtx dropped");
    }
}