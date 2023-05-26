use std::{fs::File, io::Read};

use anyhow::{Result, Context, bail};
use scoped_arena::Scope;
use sierra::{Device, Fence, Surface, Queue, ImageViewCache, DynamicGraphicsPipeline, ShaderRepr};
use winit::window::Window;

#[derive(sierra::PipelineInput)]
struct PipelineInput {
    #[sierra(set)]
    set: PipelineDescriptors
}

#[derive(sierra::Descriptors)]
struct PipelineDescriptors {
    #[sierra(uniform, stages(vertex, fragment))]
    camera_uniforms: CameraUniforms
}

#[derive(Clone, Copy, ShaderRepr)]
#[sierra(std140)]
struct CameraUniforms {
    proj: sierra::mat4,
    view: sierra::mat4
}

pub struct Renderer<'a> {
    scope: Scope<'a>,

    surface: Surface,
    device: Device,
    queue: Queue,
    graphics_pipeline: DynamicGraphicsPipeline,
    view_cache: ImageViewCache,

    fences: Vec<Option<Fence>>,
    fence_index: usize,
    non_optimal_count: u32
}

impl Renderer<'_> {
    pub const NON_OPTIMAL_LIMIT: u32 = 100;
    pub const FRAMES_IN_FLIGHT: usize = 3;

    pub fn new(
        window: &Window
    ) -> Result<Self> {
        let scope = Scope::new();
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
        let (device, queue) = physical.create_device(&features, sierra::SingleQueueQuery::GRAPHICS)?;

        let shader_module = {
            let shader_bytes = File::open("in/spirv/shaders.spv")?.bytes().try_collect::<Vec<_>>()?;
            device.create_shader_module(sierra::ShaderModuleInfo::spirv(shader_bytes))?
        };

        let mut surface = device.create_surface(window, window)?;
        surface.configure(sierra::ImageUsage::COLOR_ATTACHMENT, sierra::Format::BGRA8Srgb, sierra::PresentMode::Fifo)?;

        let pipeline_layout = PipelineInput::layout(&device)?;
        let graphics_pipeline = sierra::DynamicGraphicsPipeline::new(sierra::graphics_pipeline_desc!(
            layout: pipeline_layout.raw().clone(),
            vertex_shader: sierra::VertexShader::new(shader_module.clone(), "main_vs"),
            fragment_shader: Some(sierra::FragmentShader::new(shader_module.clone(), "main_fs")),
        ));

        let view_cache = sierra::ImageViewCache::new();
        
        Ok(Self {
            scope,

            surface,
            device,
            queue,
            graphics_pipeline,
            view_cache,

            fences: (0..Self::FRAMES_IN_FLIGHT).into_iter().map(|_| None).collect(),
            fence_index: 0,
            non_optimal_count: 0
        })
    }

    pub fn wait_idle(
        &self
    ) -> Result<()> {
        self.device.wait_idle()?;
        Ok(())
    }

    pub fn render(
        &mut self
    ) -> Result<()> {
        if let Some(fence) = &mut self.fences[self.fence_index] {
            self.device.wait_fences(&mut [fence], true)?;
            self.device.reset_fences(&mut [fence])?;
        }  
        let mut image = self.surface.acquire_image()?;
        let mut encoder = self.queue.create_encoder(&self.scope)?;

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
                        self.view_cache.make_image(image.image(), &self.device)?.clone(),
                    )
                    .clear(sierra::ClearColor(0.3, 0.1, 0.8, 1.0)),
                ),
            );
            render_pass_encoder.bind_dynamic_graphics_pipeline(&mut self.graphics_pipeline, &self.device)?;
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
        let fence = match &mut self.fences[self.fence_index] {
            Some(fence) => fence,
            None => self.fences[self.fence_index].get_or_insert(self.device.create_fence()?),
        };
        self.fence_index += 1;
        self.fence_index %= Self::FRAMES_IN_FLIGHT;

        self.queue.submit(
            &mut [(sierra::PipelineStages::COLOR_ATTACHMENT_OUTPUT, wait)],
            Some(encoder.finish()),
            &mut [signal],
            Some(fence),
            &self.scope,
        )?;
        if !image.is_optimal() {
            self.non_optimal_count += 1;
        }
        let out_of_date = match self.queue.present(image) {
            Ok(_) => false,
            Err(sierra::PresentError::OutOfDate) => true,
            Err(e) => bail!(e)
        };
        if out_of_date || self.non_optimal_count >= Self::NON_OPTIMAL_LIMIT {
            self.surface.update()?;
            self.view_cache.evict(std::u64::MAX);
            self.non_optimal_count = 0;
        }
        self.scope.reset();
        
        Ok(())
    }
}