use std::{ffi::CString, rc::Rc};

use ash::vk::{PipelineVertexInputStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PrimitiveTopology, Viewport, Rect2D, Offset2D, PipelineViewportStateCreateInfo, PipelineRasterizationStateCreateInfo, PolygonMode, CullModeFlags, FrontFace, PipelineMultisampleStateCreateInfo, SampleCountFlags, PipelineColorBlendAttachmentState, ColorComponentFlags, BlendFactor, BlendOp, LogicOp, PipelineColorBlendStateCreateInfo, PipelineLayoutCreateInfo, PipelineLayout, PipelineShaderStageCreateInfo, ShaderStageFlags, GraphicsPipelineCreateInfo, PipelineCache, Pipeline};
use anyhow::{Result, bail};

use super::{render_pass::RenderPassCtx, shader::ShaderCtx};

pub struct PipelineCtx {
    pub render_pass_ctx: Rc<RenderPassCtx>,
    pub shader_ctx: Rc<ShaderCtx>,
    pub pipeline_layout: PipelineLayout,
    pub pipeline: Pipeline
}

impl PipelineCtx {
    pub fn new(
        render_pass_ctx: Rc<RenderPassCtx>,
        shader_ctx: Rc<ShaderCtx>
    ) -> Result<PipelineCtx> {
        let entry_point_name_vs = CString::new("main_vs")?;
        let entry_point_name_fs = CString::new("main_fs")?;
        let vertex_shader_state_info = PipelineShaderStageCreateInfo::builder()
            .stage(ShaderStageFlags::VERTEX)
            .module(shader_ctx.module)
            .name(&entry_point_name_vs)
            .build();
        let fragment_shader_state_info = PipelineShaderStageCreateInfo::builder()
            .stage(ShaderStageFlags::FRAGMENT)
            .module(shader_ctx.module)
            .name(&entry_point_name_fs)
            .build();
        let shader_states_infos = [vertex_shader_state_info, fragment_shader_state_info];
        
        let vertex_input_info = PipelineVertexInputStateCreateInfo::builder().build();
        let input_assembly_info = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build();

        let viewport = Viewport {
            x: 0.0,
            y: 0.0,
            width: render_pass_ctx.swapchain_ctx.swapchain_extent.width as f32,
            height: render_pass_ctx.swapchain_ctx.swapchain_extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0
        };
        let viewports = [viewport];
        let scissor = Rect2D {
            offset: Offset2D { x: 0, y: 0 },
            extent: render_pass_ctx.swapchain_ctx.swapchain_extent
        };
        let scissors = [scissor];
        let viewport_create_info = PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors)
            .build();

        let rasteriser_create_info = PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(CullModeFlags::BACK)
            .front_face(FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0)
            .build();

        let multisampling_create_info = PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false)
            .build();

        let colour_blend_attachment = PipelineColorBlendAttachmentState::builder()
            .color_write_mask(ColorComponentFlags::RGBA)
            .blend_enable(false)
            .src_color_blend_factor(BlendFactor::ONE)
            .dst_color_blend_factor(BlendFactor::ZERO)
            .color_blend_op(BlendOp::ADD)
            .src_alpha_blend_factor(BlendFactor::ONE)
            .dst_alpha_blend_factor(BlendFactor::ZERO)
            .alpha_blend_op(BlendOp::ADD)
            .build();
        let colour_blend_attachments = [colour_blend_attachment];
        let colour_blending_info = PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(LogicOp::COPY)
            .attachments(&colour_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0])
            .build();

        let pipeline_layout_info = PipelineLayoutCreateInfo::builder().build();
        let pipeline_layout = unsafe { render_pass_ctx.swapchain_ctx.device_ctx.logical_info.device.create_pipeline_layout(&pipeline_layout_info, None)? };

        let pipeline_info = GraphicsPipelineCreateInfo::builder()
            .stages(&shader_states_infos)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_create_info)
            .rasterization_state(&rasteriser_create_info)
            .multisample_state(&multisampling_create_info)
            .color_blend_state(&colour_blending_info)
            .layout(pipeline_layout)
            .render_pass(render_pass_ctx.render_pass)
            .subpass(0)
            .build();
        let pipeline_infos = [pipeline_info];
        let maybe_pipelines = unsafe { render_pass_ctx.swapchain_ctx.device_ctx.logical_info.device.create_graphics_pipelines(PipelineCache::null(), &pipeline_infos, None) };
        let pipelines = match maybe_pipelines {
            Ok(pipelines) => pipelines,
            Err((_, result)) => bail!(result)
        };
        let pipeline = pipelines[0];

        log::debug!("PipelineCtx created");
        Ok(PipelineCtx {
            render_pass_ctx,
            shader_ctx,
            pipeline_layout,
            pipeline
        })
    }
}

impl Drop for PipelineCtx {
    fn drop(&mut self) {
        unsafe {
            self.render_pass_ctx.swapchain_ctx.device_ctx.logical_info.device.destroy_pipeline(self.pipeline, None);
            self.render_pass_ctx.swapchain_ctx.device_ctx.logical_info.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
        log::debug!("PipelineCtx dropped");
    }
}