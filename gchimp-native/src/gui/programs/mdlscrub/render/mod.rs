use std::sync::Arc;

use dem::bitvec::view;
use eframe::{
    egui::{self, PaintCallbackInfo},
    emath,
};
use egui_wgpu::wgpu;

use crate::gui::programs::mdlscrub::render::{
    camera::ScrubCamera, mdl_buffer::dynamic_buffer::WorldDynamicBuffer,
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
    pub name: String,
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
        _callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let viewport = info.viewport_in_pixels();
        let rect = info.clip_rect_in_pixels();

        // if self.name == "chain5.mdl" {
        //     let a = &viewport;

        //     println!(
        //         "viewport {} {} {} {}",
        //         a.left_px, a.top_px, a.width_px, a.height_px
        //     );
        //     let a = &rect;

        //     println!(
        //         "rect {} {} {} {}",
        //         a.left_px, a.top_px, a.width_px, a.height_px
        //     );
        //     println!("self.rect {:?}", self.rect);

        //     println!("");
        // }

        render_pass.set_viewport(
            self.rect.min.x * info.pixels_per_point,
            self.rect.min.y * info.pixels_per_point,
            // not sure why viewport.width_px doesn't just work
            self.rect.width() * info.pixels_per_point,
            self.rect.height() * info.pixels_per_point,
            0.,
            1.,
        );

        render_pass.set_scissor_rect(
            rect.left_px as u32,
            rect.top_px as u32,
            rect.width_px as u32,
            rect.height_px as u32,
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
