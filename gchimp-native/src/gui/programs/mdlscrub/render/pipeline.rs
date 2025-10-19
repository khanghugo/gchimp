use egui_wgpu::wgpu;

use crate::gui::{
    programs::mdlscrub::render::{
        camera::CameraPushConstant, mdl_buffer::MdlVertex, mvp::MvpBuffer,
        texture_array::TextureArrayBuffer,
    },
    WgpuContext,
};

#[derive(Debug)]
pub struct MdlScrubRenderer {
    pub wgpu_context: WgpuContext,
    // pub render_state: Arc<egui_wgpu::RenderState>,
    pub pipeline: wgpu::RenderPipeline,
}

impl MdlScrubRenderer {
    pub fn new(wgpu_context: WgpuContext) -> Self {
        let WgpuContext {
            device,
            queue: _,
            target_format,
        } = &wgpu_context;

        let mdlscrub_shader =
            device.create_shader_module(wgpu::include_wgsl!("./mdlscrub_shader.wgsl"));

        let mvp_bind_group_layout =
            device.create_bind_group_layout(&MvpBuffer::bind_group_layout_descriptor());

        let texture_array_bind_group_layout =
            device.create_bind_group_layout(&TextureArrayBuffer::bind_group_layout_descriptor());

        let push_constant_ranges = vec![wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX,
            range: 0..std::mem::size_of::<CameraPushConstant>() as u32,
        }];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &mvp_bind_group_layout,           // 0
                &texture_array_bind_group_layout, // 1
            ],
            push_constant_ranges: &push_constant_ranges,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("MdlSrub Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &mdlscrub_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[MdlVertex::buffer_layout()],
            },
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &mdlscrub_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: *target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self {
            wgpu_context,
            pipeline: render_pipeline,
        }
    }
}
