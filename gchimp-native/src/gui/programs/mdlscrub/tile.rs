use std::{path::PathBuf, sync::Arc};

use eframe::{
    egui::{self, Sense, Ui, Vec2},
    emath,
};

use crate::gui::programs::mdlscrub::render::{
    camera::ScrubCamera, mdl_buffer::dynamic_buffer::WorldDynamicBuffer,
    pipeline::MdlScrubRenderer, TileRenderCallback,
};

pub const SCRUB_TILE_SIZE: f32 = 96.;

#[derive(Debug)]
pub struct ScrubTile {
    pub id: egui::Id,
    pub name: String,
    pub path: PathBuf,
    pub buffer: Arc<WorldDynamicBuffer>,
    pub camera: ScrubCamera,
}

impl ScrubTile {
    pub fn view(&mut self, ui: &mut Ui, tile_renderer: &MdlScrubRenderer) {
        egui::Grid::new(self.id)
            .num_columns(1)
            .max_col_width(SCRUB_TILE_SIZE)
            .spacing([0., 0.])
            .show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::splat(SCRUB_TILE_SIZE), Sense::click());

                if ui.is_rect_visible(rect) {
                    let callback_passin = TileRenderCallback {
                        rect,
                        pipeline: Arc::new(tile_renderer.pipeline.clone()),
                        buffer: self.buffer.clone(),
                        camera: self.camera.clone(),
                    };

                    let callback = egui_wgpu::Callback::new_paint_callback(rect, callback_passin);

                    ui.painter().add(callback);
                }

                ui.label(&self.name)
            });
    }
}
