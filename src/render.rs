use std::{fs::File, io::Read};

use anyhow::{Result, Context, bail};
use scoped_arena::Scope;
use sierra::{Device, Fence, Surface, Queue, ImageViewCache, DynamicGraphicsPipeline, ShaderRepr, Buffer, BufferUsage, BufferInfo};
use winit::window::Window;

use crate::scene::{Camera, Scene};

#[derive(sierra::PipelineInput)]
struct PipelineInput {
    #[sierra(push(std430), vertex)]
    camera: CameraUniforms
}

#[derive(Clone, Copy, ShaderRepr)]
#[sierra(std430)]
struct CameraUniforms {
    proj: sierra::mat4,
    view: sierra::mat4
}

impl CameraUniforms {
    pub fn from_camera(
        camera: &Camera,
    ) -> Self {
        Self {
            proj: camera.to_projection_matrix().to_cols_array_2d().into(),
            view: camera.to_view_matrix().to_cols_array_2d().into(),
        }
    }
}

struct SceneData {
    pub vertex_buffer: Buffer,
    pub vertex_buffer_offset: u64,
    pub vertex_count: u32
}

pub struct Renderer<'a> {
    scope: Scope<'a>,

    surface: Surface,
    device: Device,
    queue: Queue,
    pipeline_layout: PipelineInputLayout,
    graphics_pipeline: DynamicGraphicsPipeline,
    view_cache: ImageViewCache,

    scene_data: SceneData,

    fences: Vec<Option<Fence>>,
    fence_index: usize,
    non_optimal_count: u32
}

impl Renderer<'_> {
    pub const NON_OPTIMAL_LIMIT: u32 = 100;
    pub const FRAMES_IN_FLIGHT: usize = 3;

    pub fn new(
        window: &Window,
        scene: &Scene
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

        let scene_data = {
            let vertex_data = bytemuck::cast_slice(&scene.vertices) as &[u8];

            let vertex_buffer = device.create_buffer_static(
                BufferInfo {
                    align: 255,
                    size: vertex_data.len() as u64,
                    usage: BufferUsage::VERTEX
                },
                vertex_data
            )?;

            SceneData {
                vertex_buffer,
                vertex_buffer_offset: 0, 
                vertex_count: scene.vertices.len() as u32
            }
        };
        
        Ok(Self {
            scope,

            surface,
            device,
            queue,
            pipeline_layout,
            graphics_pipeline,
            view_cache,

            scene_data,

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
        &mut self,
        camera: &Camera
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
            // render_pass_encoder.bind_vertex_buffers(0, &mut [(&self.scene_data.vertex_buffer, self.scene_data.vertex_buffer_offset)]);
            // render_pass_encoder.draw(0..self.scene_data.vertex_count, 0..1);
            dbg!(CameraUniforms::from_camera(camera).proj);
            render_pass_encoder.push_constants(&self.pipeline_layout, &CameraUniforms::from_camera(camera));
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