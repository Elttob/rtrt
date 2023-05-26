#![feature(iterator_try_collect)]
use std::{fs::File, io::Read};

use anyhow::{Result, bail, Context};
use scoped_arena::Scope;
use winit::{window::{WindowBuilder, Window}, event_loop::{EventLoop, ControlFlow}, event::{Event, WindowEvent}};

fn create_window() -> Result<(EventLoop<()>, Window)> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Real Time Ray Tracing")
        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
        .with_visible(false)
        .build(&event_loop)?;
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

    Ok((event_loop, window))
}

#[derive(sierra::Pass)]
#[sierra(subpass(color = target))]
#[sierra(dependency([external, color_attachment_output] => [0, color_attachment_output]))]
pub struct Main {
    #[sierra(attachment(clear = const sierra::ClearColor(0.3, 0.1, 0.8, 1.0), store = const sierra::Layout::Present))]
    target: sierra::Image,
}

#[derive(sierra::PipelineInput)]
struct PipelineInput {
    #[sierra(set)]
    descriptors: Descriptors,

    #[sierra(push(std140), compute)]
    foo: Foo,
}

#[derive(sierra::ShaderRepr)]
#[sierra(std140)]
struct Foo {
    foo: u32,
    bar: f32,
}

#[derive(sierra::Descriptors)]
struct Descriptors {
    #[sierra(buffer, vertex)]
    views: sierra::Buffer,

    #[sierra(image(storage), vertex)]
    image: sierra::Image,

    #[sierra(sampler, fragment)]
    sampler: sierra::Sampler,

    #[sierra(image(sampled), fragment)]
    albedo: sierra::ImageView,

    #[sierra(uniform, stages(vertex, fragment))]
    foo: Foo,
}

fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "DEBUG");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    log::debug!("Starting!");

    let mut scope = Scope::new();
    let (event_loop, window) = create_window()?;

    let graphics = sierra::Graphics::get_or_init()?;
    let physical = graphics.devices()?.into_iter().max_by_key(|d| d.info().kind).context("No physical device found")?;

    let features = [
        sierra::Feature::DynamicRendering,
        sierra::Feature::AccelerationStructure,
        sierra::Feature::RayTracingPipeline,
        sierra::Feature::SurfacePresentation,
        sierra::Feature::BufferDeviceAddress
    ];
    for feature in features {
        if !physical.info().features.contains(&feature) {
            bail!("Device is missing required feature: {:?}", feature);
        }
    }
    let (device, mut queue) = physical.create_device(&features, sierra::SingleQueueQuery::GRAPHICS)?;

    let shader_module = {
        let shader_bytes = File::open("in/spirv/shaders.spv")?.bytes().try_collect::<Vec<_>>()?;
        device.create_shader_module(sierra::ShaderModuleInfo::spirv(shader_bytes))?
    };

    let mut surface = device.create_surface(&window, &window)?;
    surface.configure(sierra::ImageUsage::COLOR_ATTACHMENT, sierra::Format::BGRA8Srgb, sierra::PresentMode::Fifo)?;

    let main = Main::instance();
    let pipeline_layout = PipelineInput::layout(&device)?;
    let mut graphics_pipeline = sierra::DynamicGraphicsPipeline::new(sierra::graphics_pipeline_desc!(
        layout: pipeline_layout.raw().clone(),
        vertex_shader: sierra::VertexShader::new(shader_module.clone(), "main_vs"),
        fragment_shader: Some(sierra::FragmentShader::new(shader_module.clone(), "main_fs")),
    ));

    let mut fences = [None, None, None];
    let fences_len = fences.len();
    let mut fence_index = 0;
    let non_optimal_limit = 100u32;
    let mut non_optimal_count = 0;

    let mut view_cache = sierra::ImageViewCache::new();

    window.set_visible(true);
    event_loop.run(move |event, _target, flow| {
        *flow = ControlFlow::Poll;

        (|| -> Result<()> {
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    device.wait_idle()?;
                    *flow = ControlFlow::Exit;
                },
                Event::MainEventsCleared => window.request_redraw(),
                Event::RedrawRequested(window_id) if window_id == window.id() => {
                    if let Some(fence) = &mut fences[fence_index] {
                        device.wait_fences(&mut [fence], true)?;
                        device.reset_fences(&mut [fence])?;
                    }  
                    let mut image = surface.acquire_image()?;
                    let mut encoder = queue.create_encoder(&scope)?;

                    encoder.image_barriers(
                        sierra::PipelineStages::COLOR_ATTACHMENT_OUTPUT,
                        sierra::PipelineStages::COLOR_ATTACHMENT_OUTPUT,
                        &[sierra::ImageMemoryBarrier::initialize_whole(
                            image.image(),
                            sierra::Access::COLOR_ATTACHMENT_WRITE,
                            sierra::Layout::ColorAttachmentOptimal,
                        )],
                    );

                    {
                        let mut render_pass_encoder = encoder.begin_rendering(
                            sierra::RenderingInfo::new().color(
                                &sierra::RenderingColorInfo::new(
                                    view_cache.make_image(image.image(), &device)?.clone(),
                                )
                                .clear(sierra::ClearColor(0.3, 0.1, 0.8, 1.0)),
                            ),
                        );
                        render_pass_encoder.bind_dynamic_graphics_pipeline(&mut graphics_pipeline, &device)?;
                        render_pass_encoder.push_constants(&pipeline_layout, &Foo { foo: 0, bar: 1.0 });
                        render_pass_encoder.draw(0..3, 0..1);
                    }

                    encoder.image_barriers(
                        sierra::PipelineStages::COLOR_ATTACHMENT_OUTPUT,
                        sierra::PipelineStages::TOP_OF_PIPE,
                        &[sierra::ImageMemoryBarrier::transition_whole(
                            image.image(),
                            sierra::Access::COLOR_ATTACHMENT_WRITE..sierra::Access::empty(),
                            sierra::Layout::ColorAttachmentOptimal..sierra::Layout::Present,
                        )],
                    );

                    let [wait, signal] = image.wait_signal();
                    let fence = match &mut fences[fence_index] {
                        Some(fence) => fence,
                        None => fences[fence_index].get_or_insert(device.create_fence()?),
                    };
                    fence_index += 1;
                    fence_index %= fences_len;

                    queue.submit(
                        &mut [(sierra::PipelineStages::COLOR_ATTACHMENT_OUTPUT, wait)],
                        Some(encoder.finish()),
                        &mut [signal],
                        Some(fence),
                        &scope,
                    )?;
                    if !image.is_optimal() {
                        non_optimal_count += 1;
                    }
                    let out_of_date = match queue.present(image) {
                        Ok(_) => false,
                        Err(sierra::PresentError::OutOfDate) => true,
                        Err(e) => bail!(e)
                    };
                    if out_of_date || non_optimal_count >= non_optimal_limit {
                        surface.update()?;
                        view_cache.evict(std::u64::MAX);
                        non_optimal_count = 0;
                    }
                    scope.reset();
                },
                _ => {}
            };
            Ok(())
        })().unwrap()
    });
}