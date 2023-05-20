use std::sync::Arc;

use anyhow::Result;
use vulkano::{VulkanLibrary, instance::{Instance, InstanceCreateInfo}, device::{DeviceExtensions, Device, DeviceCreateInfo, QueueFlags, physical::{PhysicalDeviceType, PhysicalDevice}, QueueCreateInfo, Queue}, swapchain::{Swapchain, SwapchainCreateInfo, Surface, AcquireError, SwapchainPresentInfo}, image::{ImageUsage, SwapchainImage, view::ImageView}, render_pass::{RenderPass, Framebuffer, FramebufferCreateInfo, Subpass}, sync::{future::FenceSignalFuture, self, GpuFuture, FlushError}, pipeline::{graphics::{viewport::{Viewport, ViewportState}, input_assembly::InputAssemblyState, vertex_input::Vertex}, GraphicsPipeline}, shader::ShaderModule, command_buffer::{allocator::StandardCommandBufferAllocator}};
use winit::{window::{Window}, dpi::PhysicalSize};

use crate::{scene::{GeoVertex, Scene}};

pub struct DeviceCtx {
    surface: Arc<Surface>,
    physical_device: Arc<PhysicalDevice>,
    pub device: Arc<Device>,
    queue: Arc<Queue>
}

impl DeviceCtx {
    fn select_physical_device(
        instance: &Arc<Instance>,
        surface: &Arc<Surface>,
        device_extensions: &DeviceExtensions,
    ) -> (Arc<PhysicalDevice>, u32) {
        instance
            .enumerate_physical_devices()
            .expect("could not enumerate devices")
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    // Find the first first queue family that is suitable.
                    // If none is found, `None` is returned to `filter_map`,
                    // which disqualifies this physical device.
                    .position(|(i, q)| {
                        q.queue_flags.contains(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|q| (p, q as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
    
                // Note that there exists `PhysicalDeviceType::Other`, however,
                // `PhysicalDeviceType` is a non-exhaustive enum. Thus, one should
                // match wildcard `_` to catch all unknown device types.
                _ => 4,
            })
            .expect("no device available")
    }

    pub fn new(
        window: Arc<Window>
    ) -> Result<Self> {
        let library = VulkanLibrary::new()?;
        let required_extensions = vulkano_win::required_extensions(&library);
        let instance = Instance::new(
            library, 
            InstanceCreateInfo {
                enabled_extensions: required_extensions,
                ..Default::default()
            }
        )?;
        let surface = vulkano_win::create_surface_from_winit(window, instance.clone())?;
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let (physical_device, queue_family_index) = Self::select_physical_device(&instance, &surface, &device_extensions);
        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            },
        ).expect("failed to create device");
        let queue = queues.next().unwrap();

        Ok(Self {
            surface,
            physical_device,
            device,
            queue
        })
    }
}

pub struct PresentCtx {
    swapchain: Arc<Swapchain>,
    swapchain_images: Vec<Arc<SwapchainImage>>,
    swapchain_dimensions: PhysicalSize<u32>,
    swapchain_fences: Vec<Option<Arc<FenceSignalFuture<Box<dyn GpuFuture>>>>>,
    last_image_index: u32,

    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    viewport: Viewport,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    pipeline: Arc<GraphicsPipeline>,
    command_buffer_allocator: StandardCommandBufferAllocator
}

pub struct RenderStatus {
    pub rendered: bool,
    pub needs_recreate: bool
}

impl PresentCtx {
    fn get_render_pass(
        device: Arc<Device>,
        swapchain: Arc<Swapchain>
    ) -> Arc<RenderPass> {
        vulkano::single_pass_renderpass!(
            device,
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(), // set the format the same as the swapchain
                    samples: 1,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        ).unwrap()
    }
    
    fn get_framebuffers(
        images: &[Arc<SwapchainImage>],
        render_pass: Arc<RenderPass>,
    ) -> Vec<Arc<Framebuffer>> {
        images.iter().map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .unwrap()
        }).collect::<Vec<_>>()
    }

    fn get_pipeline(
        device: Arc<Device>,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(GeoVertex::per_vertex())
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .render_pass(Subpass::from(render_pass, 0).unwrap())
            .build(device)
            .unwrap()
    }

    // fn get_command_buffers(
    //     command_buffer_allocator: &StandardCommandBufferAllocator,
    //     queue: &Arc<Queue>,
    //     pipeline: &Arc<GraphicsPipeline>,
    //     framebuffers: &Vec<Arc<Framebuffer>>,
    //     vertex_buffer: &Subbuffer<[GeoVertex]>,
    //     push_constants: crate::vs::PushConstants
    // ) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
    //     framebuffers.iter().map(|framebuffer| {
    //         let mut builder = AutoCommandBufferBuilder::primary(
    //             command_buffer_allocator,
    //             queue.queue_family_index(),
    //             CommandBufferUsage::MultipleSubmit, // don't forget to write the correct buffer usage
    //         )
    //         .unwrap();
    
    //         builder.begin_render_pass(
    //             RenderPassBeginInfo {
    //                 clear_values: vec![Some([0.3, 0.5, 0.7, 1.0].into())],
    //                 ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
    //             },
    //             SubpassContents::Inline,
    //         )
    //         .unwrap()
    //         .bind_pipeline_graphics(pipeline.clone())
    //         .bind_vertex_buffers(0, vertex_buffer.clone())
    //         .push_constants(pipeline.layout().clone(), 0, push_constants)
    //         .draw(vertex_buffer.len() as u32, 1, 0, 0)
    //         .unwrap()
    //         .end_render_pass()
    //         .unwrap();
    
    //         Arc::new(builder.build().unwrap())
    //     })
    //     .collect()
    // }

    pub fn new(
        device_ctx: &DeviceCtx,
        dimensions: PhysicalSize<u32>
    ) -> Result<Self> {
        let caps = device_ctx.physical_device.surface_capabilities(&device_ctx.surface, Default::default())?;
        let composite_alpha = caps.supported_composite_alpha.into_iter().next().ok_or(anyhow::anyhow!("no composite alpha found"))?;
        let image_format = Some(device_ctx.physical_device.surface_formats(&device_ctx.surface, Default::default())?[0].0);
        let (swapchain, swapchain_images) = Swapchain::new(
            device_ctx.device.clone(),
            device_ctx.surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count,
                image_format,
                image_extent: dimensions.into(),
                image_usage : ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                ..Default::default()
            }
        )?;
        let swapchain_fences = vec![None; swapchain_images.len()];

        let render_pass = Self::get_render_pass(device_ctx.device.clone(), swapchain.clone());
        let framebuffers = Self::get_framebuffers(&swapchain_images, render_pass.clone());

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: dimensions.into(),
            depth_range: 0.0..1.0,
        };
        let vs = crate::vs::load(device_ctx.device.clone())?;
        let fs = crate::fs::load(device_ctx.device.clone())?;
        let pipeline = Self::get_pipeline(
            device_ctx.device.clone(),
            vs.clone(),
            fs.clone(),
            render_pass.clone(),
            viewport.clone(),
        );

        let command_buffer_allocator = StandardCommandBufferAllocator::new(device_ctx.device.clone(), Default::default());

        Ok(Self {
            swapchain,
            swapchain_images,
            swapchain_dimensions: dimensions,
            swapchain_fences,
            last_image_index: 0,
            render_pass,
            framebuffers,
            viewport,
            vs,
            fs,
            pipeline,
            command_buffer_allocator,
        })
    }

    pub fn recreate_swapchain(
        &mut self,
        device_ctx: &DeviceCtx,
        new_dimensions: PhysicalSize<u32>
    ) -> Result<()> {
        let is_resize = self.swapchain_dimensions != new_dimensions;

        let (new_swapchain, new_images) = self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: new_dimensions.into(),
            ..self.swapchain.create_info()
        })?;
        let new_framebuffers = Self::get_framebuffers(&new_images, self.render_pass.clone());
        
        self.swapchain = new_swapchain;
        self.swapchain_images = new_images;
        self.swapchain_dimensions = new_dimensions;
        self.framebuffers = new_framebuffers;
        if is_resize {
            self.viewport.dimensions = new_dimensions.into();
            self.pipeline = Self::get_pipeline(
                device_ctx.device.clone(),
                self.vs.clone(),
                self.fs.clone(),
                self.render_pass.clone(),
                self.viewport.clone(),
            );
        }

        Ok(())
    }

    // Returns true if the swapchain should be recreated
    pub fn render(
        &mut self,
        device_ctx: &DeviceCtx,
        scene: &Scene
    ) -> Result<RenderStatus> {
        let mut return_status = RenderStatus {
            rendered: true,
            needs_recreate: false
        };

        let (next_image_index, suboptimal, acquire_future) = match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => return Ok(RenderStatus { rendered: false, needs_recreate: true }),
            Err(e) => panic!("failed to acquire next image: {e}"),
        };
        if suboptimal {
            return_status.needs_recreate = true;
        }
        if let Some(image_fence) = &self.swapchain_fences[next_image_index as usize] {
            image_fence.wait(None).unwrap();
        }
        let command_buffer = scene.build_command_buffer(&self.command_buffer_allocator, &device_ctx.queue, &self.pipeline, &self.framebuffers[next_image_index as usize]);
        let future = match self.swapchain_fences[self.last_image_index as usize].clone() {
            None => {
                let mut now = sync::now(device_ctx.device.clone());
                now.cleanup_finished();
                now.boxed()
            }
            Some(fence) => fence.boxed(),
        };

        let future = future
            .join(acquire_future)
            .then_execute(device_ctx.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                device_ctx.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), next_image_index),
            );

        let future = (Box::new(future) as Box<dyn GpuFuture>).then_signal_fence_and_flush();
        self.swapchain_fences[next_image_index as usize] = match future {
            Ok(value) => Some(Arc::new(value)),
            Err(FlushError::OutOfDate) => {
                return_status.needs_recreate = true;
                None
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                None
            }
        };
        self.last_image_index = next_image_index;

        Ok(return_status)
    }
}