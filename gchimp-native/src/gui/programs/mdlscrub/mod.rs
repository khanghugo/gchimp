use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use eframe::egui::{self, scroll_area::ScrollSource, ScrollArea, Ui};
use walkdir::WalkDir;

use crate::gui::{
    programs::mdlscrub::{
        render::{camera::ScrubCamera, pipeline::MdlScrubRenderer},
        tile::{ScrubTile, SCRUB_TILE_SIZE},
    },
    TabProgram,
};

pub mod render;
mod tile;

#[derive(Debug)]
pub struct MdlScrub {
    renderer: Arc<MdlScrubRenderer>,
    game_path: PathBuf,
    tiles: Vec<ScrubTile>,
}

impl MdlScrub {
    pub fn new(renderer: Arc<MdlScrubRenderer>) -> Self {
        Self {
            renderer,
            game_path: Default::default(),
            tiles: Default::default(),
        }
    }

    fn menu_open(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            let mdl_paths = get_mdl_paths_recursively(&path);

            let mut tiles: Vec<ScrubTile> = mdl_paths
                .into_iter()
                .map(|path| {
                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    let mdl = mdl::Mdl::open_from_file(path.as_path()).unwrap();

                    let buffer = Arc::new(self.renderer.load_dynamic_world(file_name, &mdl, 0));

                    let mut camera = ScrubCamera::default();

                    camera.pos = cgmath::point3(0., -100., 100.);

                    ScrubTile {
                        id: egui::Id::new(path.display().to_string()),
                        name: file_name.to_string(),
                        path,
                        buffer,
                        camera,
                    }
                })
                .collect();

            // TODO could make it case insensitive
            tiles.sort_by(|a, b| a.name.cmp(&b.name));

            self.tiles = tiles;
        }
    }

    fn scrub_grid(&mut self, ui: &mut Ui) {
        let tile_count = self.tiles.len();

        let image_tile_size = SCRUB_TILE_SIZE * ui.ctx().options(|options| options.zoom_factor);
        let tile_per_row = ((ui.min_size().x / image_tile_size).floor() as usize).max(4);
        let row_height = 2. // margin
            + 18. // 1 labels
            + image_tile_size;

        // let is_search_enabled = self.instances[instance_index].search.enable;
        // let search_text = self.instances[instance_index].search.text.to_lowercase();
        // let filtered_tiles = (0..tile_count)
        //     .filter(|&texture_tile| {
        //         if is_search_enabled {
        //             self.instances[instance_index].texture_tiles[texture_tile]
        //                 .name()
        //                 .to_lowercase()
        //                 .contains(search_text.as_str())
        //         } else {
        //             true
        //         }
        //     })
        //     .collect::<Vec<usize>>();

        let total_rows = tile_count.div_ceil(tile_per_row);

        ScrollArea::vertical()
            .scroll_source(ScrollSource::MOUSE_WHEEL)
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                // each row is one grid of grids
                row_range.for_each(|row| {
                    egui::Grid::new(format!("mdlscrub_grid_row{}", row))
                        .num_columns(tile_per_row)
                        .spacing([2., 2.])
                        .show(ui, |ui| {
                            self.tiles
                                .chunks_mut(tile_per_row)
                                .nth(row)
                                .expect("invalid row")
                                .iter_mut()
                                .for_each(|tile| {
                                    tile.view(ui, &self.renderer);
                                });
                        });
                });
            });
    }
}

impl TabProgram for MdlScrub {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "MdlScrub".into()
    }

    fn tab_ui(&mut self, ui: &mut Ui) -> egui_tiles::UiResponse {
        ui.separator();

        ui.menu_button("Menu", |ui| {
            if ui.button("Open").clicked() {
                self.menu_open();
                ui.close();
            }
        });

        ui.separator();

        self.scrub_grid(ui);

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}

fn get_mdl_paths_recursively(path: &Path) -> Vec<PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().is_some_and(|x| x == "mdl"))
        .map(|entry| entry.into_path())
        .collect()
}
