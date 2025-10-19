use std::sync::Arc;

use eframe::{
    egui::{self, PaintCallbackInfo},
    emath,
};
use egui_wgpu::wgpu;

use crate::gui::programs::mdlscrub::render::{
    camera::ScrubCamera,
    mdl_buffer::{dynamic_buffer::WorldDynamicBuffer, MdlVertexBuffer},
    mvp::MvpBuffer,
    texture_array::TextureArrayBuffer,
};

pub mod camera;
pub mod mdl_buffer;
mod mvp;
pub mod pipeline;
mod texture_array;

pub struct TileRenderCallback {
    pub rect: emath::Rect,
    pub pipeline: Arc<wgpu::RenderPipeline>,
    pub buffer: Arc<WorldDynamicBuffer>,
    pub camera: ScrubCamera,
}

impl egui_wgpu::CallbackTrait for TileRenderCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        vec![]
    }
    fn paint(
        &self,
        info: PaintCallbackInfo,
        render_pass: &mut egui_wgpu::wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let vp = info.viewport_in_pixels();
        render_pass.set_viewport(
            vp.left_px as f32,
            vp.top_px as f32,
            vp.width_px as f32,
            vp.height_px as f32,
            0.,
            1.,
        );
        render_pass.set_scissor_rect(
            vp.left_px as u32,
            vp.top_px as u32,
            vp.width_px as u32,
            vp.height_px as u32,
        );

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.buffer.mvp_buffer.bind_group, &[]);

        let proj_view = self.camera.proj_view();
        let proj_view_cast: [[f32; 4]; 4] = proj_view.into();

        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX,
            0,
            bytemuck::bytes_of(&proj_view_cast),
        );

        // usually only runs 1 time
        self.buffer.opaque.iter().for_each(|batch| {
            render_pass.set_bind_group(
                1,
                &self.buffer.textures[batch.texture_array_index].bind_group,
                &[],
            );

            render_pass.set_vertex_buffer(0, batch.vertex_buffer.slice(..));
            render_pass.set_index_buffer(batch.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            render_pass.draw_indexed(0..batch.index_count as u32, 0, 0..1);
        });
    }
}
