use ash::{extensions::khr::Swapchain, vk::{SwapchainKHR, Format, Extent2D, Image, SurfaceCapabilitiesKHR, SurfaceFormatKHR, PresentModeKHR, ColorSpaceKHR, SwapchainCreateInfoKHR, ImageUsageFlags, SharingMode, CompositeAlphaFlagsKHR}};
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
        let support_details = &self.physical_device.swapchain_support_details;
        let format = select_surface_format(&support_details.formats);
        let present_mode = select_surface_present_mode(&support_details.present_modes);
        let extent = select_extent(support_details.capabilities, preferred_extent);
        let image_count = {
            let max = support_details.capabilities.max_image_count;
            let preferred = support_details.capabilities.min_image_count + 1;
            if max == 0 || preferred <= max { preferred } else { max }
        };
        let image_sharing_mode = if self.physical_device.dedup_family_indices.len() > 1 { SharingMode::CONCURRENT } else { SharingMode::EXCLUSIVE };
        let create_info = SwapchainCreateInfoKHR::builder()
            .surface(self.surface_ctx.surface_khr)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(image_sharing_mode)
            .queue_family_indices(&self.physical_device.dedup_family_indices)
            .pre_transform(support_details.capabilities.current_transform)
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .build();
        let swapchain = Swapchain::new(&self.surface_ctx.instance_ctx.instance, &self.logical_device.device);
        let swapchain_khr = unsafe { swapchain.create_swapchain(&create_info, None)? };
        let images = unsafe { swapchain.get_swapchain_images(swapchain_khr)? };

        log::debug!("SwapchainCtx created (format: {:?}, clr space: {:?}, pres mode: {:?}, extent: {:?}, count: {})", format.format, format.color_space, present_mode, extent, image_count);
        Ok(SwapchainCtx {
            device_ctx: self,
            swapchain,
            swapchain_khr,
            images,
            swapchain_image_format: format.format,
            swapchain_extent: extent
        })
    }
}

impl Drop for SwapchainCtx<'_, '_, '_, '_> {
    fn drop(&mut self) {
        unsafe {
            self.swapchain.destroy_swapchain(self.swapchain_khr, None);
        }
        log::debug!("SwapchainCtx dropped");
    }
}