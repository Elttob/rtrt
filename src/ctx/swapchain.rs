use ash::{extensions::khr::Swapchain, vk::{SwapchainKHR, Format, Extent2D, Image, SurfaceCapabilitiesKHR, SurfaceFormatKHR, PresentModeKHR, ColorSpaceKHR}};
use anyhow::Result;
use super::device::DeviceCtx;

fn select_surface_format(
    available_formats: &[SurfaceFormatKHR],
) -> SurfaceFormatKHR {
    if available_formats.len() == 1 && available_formats[0].format == Format::UNDEFINED {
        SurfaceFormatKHR {
            format: Format::B8G8R8A8_UNORM,
            color_space: ColorSpaceKHR::SRGB_NONLINEAR
        }
    } else {
        *available_formats.iter()
        .find(|x| x.format == Format::B8G8R8A8_UNORM && x.color_space == ColorSpaceKHR::SRGB_NONLINEAR)
        .unwrap_or(&available_formats[0])
    }
}

fn select_surface_present_mode(
    available_present_modes: &[PresentModeKHR],
) -> PresentModeKHR {
    if available_present_modes.contains(&PresentModeKHR::MAILBOX) {
        PresentModeKHR::MAILBOX
    } else if available_present_modes.contains(&PresentModeKHR::FIFO) {
        PresentModeKHR::FIFO
    } else {
        PresentModeKHR::IMMEDIATE
    }
}

fn select_extent(
    capabilities: SurfaceCapabilitiesKHR,
    preferred_extent: Extent2D
) -> Extent2D {
    // current_extent = (0xFFFFFFFF, 0xFFFFFFFF) indicates the surface size
    // will be determined by the extent of a swapchain targeting the surface
    if capabilities.current_extent.width != std::u32::MAX {
        capabilities.current_extent
    } else {
        let (min, max) = (capabilities.min_image_extent, capabilities.max_image_extent);
        Extent2D {
            width: preferred_extent.width.clamp(min.width, max.width),
            height: preferred_extent.height.clamp(min.height, max.height)
        }
    }
}

pub struct SwapchainCtx<'dev, 'srf, 'ins, 'en> {
    pub device_ctx: &'dev DeviceCtx<'srf, 'ins, 'en>,
    pub swapchain: Swapchain,
    pub swapchain_khr: SwapchainKHR,
    pub images: Vec<Image>,
    pub swapchain_image_format: Format,
    pub swapchain_extent: Extent2D
}

impl<'srf, 'ins, 'en> DeviceCtx<'srf, 'ins, 'en> {
    pub fn create_swapchain_ctx(
        &self,
        preferred_extent: Extent2D
    ) -> Result<SwapchainCtx> {
        let details = self.swapchain_support_details()?;
        let format = select_surface_format(&details.formats);
        let present_mode = select_surface_present_mode(&details.present_modes);
        let extent = select_extent(details.capabilities, preferred_extent);

        log::debug!("SwapchainCtx created");
        Ok(SwapchainCtx {
            device_ctx: self,
            swapchain,
            swapchain_khr,
            images,
            swapchain_image_format,
            swapchain_extent
        })
    }

    fn swapchain_support_details(
        &self
    ) -> Result<SwapchainSupportDetails> {
        let (surface, surface_khr, physical_device) = (self.surface_ctx.surface, self.surface_ctx.surface_khr, self.physical_device);
        let capabilities = unsafe { surface.get_physical_device_surface_capabilities(physical_device, surface_khr)? };
        let formats = unsafe { surface.get_physical_device_surface_formats(physical_device, surface_khr)? };
        let present_modes = unsafe { surface.get_physical_device_surface_present_modes(physical_device, surface_khr)? };
        Ok(SwapchainSupportDetails {
            capabilities,
            formats,
            present_modes,
        })
    }
}

impl Drop for SwapchainCtx<'_, '_, '_, '_> {
    fn drop(&mut self) {
        log::debug!("SwapchainCtx dropped");
    }
}

// SUPPORTING TYPES

struct SwapchainSupportDetails {
    capabilities: SurfaceCapabilitiesKHR,
    formats: Vec<SurfaceFormatKHR>,
    present_modes: Vec<PresentModeKHR>,
}