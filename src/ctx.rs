use std::sync::Arc;

use anyhow::Result;
use glam::vec2;
use vulkano_win::VkSurfaceBuild;
use vulkano::{VulkanLibrary, instance::{Instance, InstanceCreateInfo}, device::{DeviceExtensions, Device, DeviceCreateInfo, QueueFlags, physical::{PhysicalDeviceType, PhysicalDevice}, QueueCreateInfo, Queue}, swapchain::{Swapchain, SwapchainCreateInfo, Surface, SwapchainCreationError, AcquireError, SwapchainPresentInfo}, image::{ImageUsage, SwapchainImage, view::ImageView}, render_pass::{RenderPass, Framebuffer, FramebufferCreateInfo, Subpass}, sync::{future::FenceSignalFuture, self, GpuFuture, FlushError}, pipeline::{graphics::{viewport::{Viewport, ViewportState}, input_assembly::InputAssemblyState, vertex_input::Vertex}, GraphicsPipeline, Pipeline}, shader::ShaderModule, buffer::{BufferContents, Subbuffer}, command_buffer::{allocator::StandardCommandBufferAllocator, PrimaryAutoCommandBuffer, AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents}};
use winit::{event_loop::{EventLoop, ControlFlow}, window::{WindowBuilder, Window}, event::{Event, WindowEvent}};

use crate::input::Input;

pub struct DeviceCtx {
    event_loop: EventLoop<()>,
    surface: Arc<Surface>,
    window: Arc<Window>,

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

    pub fn new() -> Result<Self> {
        let library = VulkanLibrary::new()?;
        let required_extensions = vulkano_win::required_extensions(&library);
        let instance = Instance::new(
            library, 
            InstanceCreateInfo {
                enabled_extensions: required_extensions,
                ..Default::default()
            }
        )?;

        let event_loop = EventLoop::new();
        let surface = WindowBuilder::new()
            .with_title("Real Time Ray Tracing")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .with_visible(false)
            .build_vk_surface(&event_loop, instance.clone())
            .unwrap();
        let window = surface.object().unwrap().clone().downcast::<Window>().unwrap();
        let monitor = window.current_monitor().unwrap_or(
            window.primary_monitor().unwrap_or(
                window.available_monitors().next().expect("Couldn't find a suitable monitor.")
            )
        );
        let monitor_size = monitor.size();
        let window_size = window.outer_size();
        window.set_outer_position(winit::dpi::PhysicalPosition::new(
            (monitor_size.width - window_size.width) / 2,
            (monitor_size.height - window_size.height) / 2
        ));
        window.set_visible(true);

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
            event_loop,
            surface,
            window,

            physical_device,
            device,
            queue
        })
    }
}

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct MyVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub normal: [f32; 3],
}

pub struct PresentCtx {
    swapchain: Arc<Swapchain>,
    swapchain_images: Vec<Arc<SwapchainImage>>,
    render_pass: Arc<RenderPass>,
    viewport: Viewport,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    command_buffer_allocator: StandardCommandBufferAllocator,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
    push_constants: crate::vs::PushConstants
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
            .vertex_input_state(MyVertex::per_vertex())
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .render_pass(Subpass::from(render_pass, 0).unwrap())
            .build(device)
            .unwrap()
    }

    fn get_command_buffers(
        command_buffer_allocator: &StandardCommandBufferAllocator,
        queue: &Arc<Queue>,
        pipeline: &Arc<GraphicsPipeline>,
        framebuffers: &Vec<Arc<Framebuffer>>,
        vertex_buffer: &Subbuffer<[MyVertex]>,
        push_constants: crate::vs::PushConstants
    ) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
        framebuffers.iter().map(|framebuffer| {
            let mut builder = AutoCommandBufferBuilder::primary(
                command_buffer_allocator,
                queue.queue_family_index(),
                CommandBufferUsage::MultipleSubmit, // don't forget to write the correct buffer usage
            )
            .unwrap();
    
            builder.begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.3, 0.5, 0.7, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                },
                SubpassContents::Inline,
            )
            .unwrap()
            .bind_pipeline_graphics(pipeline.clone())
            .bind_vertex_buffers(0, vertex_buffer.clone())
            .push_constants(pipeline.layout().clone(), 0, push_constants)
            .draw(vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap()
            .end_render_pass()
            .unwrap();
    
            Arc::new(builder.build().unwrap())
        })
        .collect()
    }

    pub fn new(
        device_ctx: &DeviceCtx,
        vertex_buffer: &Subbuffer<[MyVertex]>,
    ) -> Result<Self> {
        let caps = device_ctx.physical_device.surface_capabilities(&device_ctx.surface, Default::default())?;
        let dimensions = device_ctx.window.inner_size();
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

        let render_pass = Self::get_render_pass(device_ctx.device.clone(), swapchain.clone());
        let framebuffers = Self::get_framebuffers(&swapchain_images, render_pass.clone());

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: device_ctx.window.inner_size().into(),
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
        let push_constants = crate::vs::PushConstants {
            proj: 
                (glam::Mat4::from_scale(glam::vec3(1.0, -1.0, 1.0)) * glam::Mat4::perspective_lh(1.5, dimensions.width as f32 / dimensions.height as f32, 0.1, 100.0))
                .to_cols_array_2d(),
            view: 
                (glam::Mat4::look_at_lh(glam::vec3(1.0, 1.0, 1.0), glam::vec3(0.0, 0.0, 0.0), glam::vec3(0.0, 1.0, 0.0)))
                .to_cols_array_2d()
        };

        let command_buffer_allocator = StandardCommandBufferAllocator::new(device_ctx.device.clone(), Default::default());
        let command_buffers = Self::get_command_buffers(
            &command_buffer_allocator,
            &device_ctx.queue,
            &pipeline,
            &framebuffers,
            vertex_buffer,
            push_constants.clone()
        );

        Ok(Self {
            swapchain,
            swapchain_images,
            render_pass,
            viewport,
            vs,
            fs,
            command_buffer_allocator,
            command_buffers,
            push_constants
        })
    }

    pub fn run(
        mut self,
        device_ctx: DeviceCtx,
        vertex_buffer: Subbuffer<[MyVertex]>
    ) {
        let mut window_resized = false;
        let mut cursor_over_window = false;
        let mut recreate_swapchain = false;

        let frames_in_flight = self.swapchain_images.len();
        let mut fences: Vec<Option<Arc<FenceSignalFuture<_>>>> = vec![None; frames_in_flight];
        let mut previous_fence_i = 0;

        let mut input = Input::new(vec2(0.0, 0.0));

        device_ctx.event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent { ref event, window_id } if window_id == device_ctx.window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    window_resized = true;
                },
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    window_resized = true;
                },
                WindowEvent::CursorEntered { .. } => {
                    cursor_over_window = true;
                },
                WindowEvent::CursorLeft { .. } => {
                    cursor_over_window = false;
                },
                _ => {
                    if device_ctx.window.has_focus() && cursor_over_window {
                        input.process_window_events(event);
                    }
                }
            },
            Event::DeviceEvent { ref event, .. } => {
                if device_ctx.window.has_focus() && cursor_over_window {
                    input.process_device_events(event);
                    let window_size = device_ctx.window.inner_size();
                    device_ctx.window.set_cursor_position(winit::dpi::PhysicalPosition::new(
                        window_size.width / 2,
                        window_size.height / 2
                    )).expect("Platform does not support setting the cursor position");
                }
            },



            Event::MainEventsCleared => {
                if window_resized || recreate_swapchain {
                    recreate_swapchain = false;

                    let new_dimensions = device_ctx.window.inner_size();
                    self.push_constants.proj = 
                        (glam::Mat4::from_scale(glam::vec3(1.0, -1.0, 1.0)) * glam::Mat4::perspective_lh(1.5, new_dimensions.width as f32 / new_dimensions.height as f32, 0.1, 100.0))
                        .to_cols_array_2d();

                    let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
                        image_extent: new_dimensions.into(),
                        ..self.swapchain.create_info()
                    }) {
                        Ok(r) => r,
                        Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                        Err(e) => panic!("failed to recreate swapchain: {e}"),
                    };
                    self.swapchain = new_swapchain;
                    let new_framebuffers = Self::get_framebuffers(&new_images, self.render_pass.clone());

                    if window_resized {
                        window_resized = false;

                        self.viewport.dimensions = new_dimensions.into();
                        let new_pipeline = Self::get_pipeline(
                            device_ctx.device.clone(),
                            self.vs.clone(),
                            self.fs.clone(),
                            self.render_pass.clone(),
                            self.viewport.clone(),
                        );
                        self.command_buffers = Self::get_command_buffers(
                            &self.command_buffer_allocator,
                            &device_ctx.queue,
                            &new_pipeline,
                            &new_framebuffers,
                            &vertex_buffer,
                            self.push_constants.clone()
                        );
                    }
                }

                let (image_i, suboptimal, acquire_future) =
                    match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("failed to acquire next image: {e}"),
                    };

                if suboptimal {
                    recreate_swapchain = true;
                }

                // wait for the fence related to this image to finish (normally this would be the oldest fence)
                if let Some(image_fence) = &fences[image_i as usize] {
                    image_fence.wait(None).unwrap();
                }

                let previous_future = match fences[previous_fence_i as usize].clone() {
                    // Create a NowFuture
                    None => {
                        let mut now = sync::now(device_ctx.device.clone());
                        now.cleanup_finished();
                        now.boxed()
                    }
                    // Use the existing FenceSignalFuture
                    Some(fence) => fence.boxed(),
                };

                let future = previous_future
                    .join(acquire_future)
                    .then_execute(device_ctx.queue.clone(), self.command_buffers[image_i as usize].clone())
                    .unwrap()
                    .then_swapchain_present(
                        device_ctx.queue.clone(),
                        SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
                    )
                    .then_signal_fence_and_flush();

                fences[image_i as usize] = match future {
                    Ok(value) => Some(Arc::new(value)),
                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        None
                    }
                    Err(e) => {
                        println!("failed to flush future: {e}");
                        None
                    }
                };

                previous_fence_i = image_i;
            }
            _ => (),
        });
    }
}