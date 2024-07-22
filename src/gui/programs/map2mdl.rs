use std::{path::PathBuf, thread};

use eframe::egui::{self, ScrollArea};

use crate::{
    config::Config,
    gui::{
        constants::{PROGRAM_HEIGHT, PROGRAM_WIDTH},
        utils::preview_file_being_dropped,
        TabProgram,
    },
    modules::map2mdl::{Map2Mdl, Map2MdlOptions, Map2MdlSync, GCHIMP_MAP2MDL_ENTITY_NAME},
};

pub struct Map2MdlGui {
    app_config: Config,
    map: String,
    entity: String,
    use_entity: bool,
    options: Map2MdlOptions,
    sync: Map2MdlSync,
}

impl Map2MdlGui {
    pub fn new(app_config: Config) -> Self {
        Self {
            app_config,
            map: Default::default(),
            entity: Default::default(),
            use_entity: false,
            options: Map2MdlOptions::default(),
            sync: Map2MdlSync::default(),
        }
    }

    fn run(&mut self) {
        let Config {
            studiomdl,
            crowbar: _,
            no_vtf: _,
            #[cfg(target_os = "linux")]
            wineprefix,
        } = self.app_config.clone();

        let Map2MdlOptions {
            auto_pickup_wad,
            export_texture,
            move_to_origin,
            ignore_nodraw,
            marked_entity,
            ..
        } = self.options;
        let entity = self.entity.clone();
        let map = self.map.clone();
        let use_entity = self.use_entity;

        let sync = self.sync.clone();

        thread::spawn(move || {
            let mut binding = Map2Mdl::default();
            binding
                .auto_pickup_wad(auto_pickup_wad)
                .move_to_origin(move_to_origin)
                .export_texture(export_texture)
                .ignore_nodraw(ignore_nodraw)
                .studiomdl(PathBuf::from(&studiomdl).as_path())
                .marked_entity(marked_entity)
                .sync(sync.clone());

            if use_entity {
                binding.entity(&entity);
            } else {
                binding.map(&map);
            };

            #[cfg(target_os = "linux")]
            binding.wineprefix(wineprefix.as_ref().unwrap());

            if let Err(err) = binding.work() {
                *sync.stdout().lock().unwrap() = err.to_string();
            } else {
                let mut ok_text = "OK".to_string();

                if use_entity {
                    ok_text += &("\n".to_owned()
                        + "Model is saved as map2mdl.mdl at "
                        + studiomdl.replace("studiomdl.exe", "").as_str());
                }

                *sync.stdout().lock().unwrap() = ok_text;
            }
        });
    }
}

impl TabProgram for Map2MdlGui {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "Map2Mdl".into()
    }

    fn tab_ui(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.separator();

        ui.add_enabled_ui(true, |ui| {
            egui::Grid::new("map2mdl grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Map:");
                    ui.add_enabled_ui(!self.use_entity, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.map).hint_text("Choose .map file"),
                        );
                    });
                    if ui.button("Add").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            if path.extension().is_some_and(|ext| ext == "map") {
                                self.map = path.display().to_string();
                                self.use_entity = false;
                            }
                        }
                    }

                    ui.end_row();
                    ui.checkbox(&mut self.use_entity, "Entity");
                    ui.add_enabled_ui(self.use_entity, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.entity)
                                .hint_text("Worldbrush entity copied from TB"),
                        );
                    });
                    if ui.button("Clear").clicked() {
                        self.entity.clear();
                    }
                })
        });
        ui.separator();
        ui.label("Options:");

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.options.auto_pickup_wad, "Auto pickup WADs").on_hover_text("Look for WAD files from \"wad\" key in the map file or worldbrush entity");
            ui.checkbox(&mut self.options.export_texture, "Export textures").on_hover_text("Export textures into the map file folder OR studiomdl.exe folder if converting entity");
            ui.checkbox(&mut self.options.ignore_nodraw, "Skip nodraw textures").on_hover_text("NULL, CLIP, ...");
        });

        ui.horizontal(|ui| {
            ui.checkbox(
                &mut self.options.marked_entity,
                "Only convert marked entity",
            )
            .on_hover_text(format!(
                "Only convert brush entities {} and this would modify the original map file",
                GCHIMP_MAP2MDL_ENTITY_NAME
            ));
            ui.checkbox(&mut self.options.move_to_origin, "Center the model")
                .on_hover_text("The center of the model is the origin");
        });

        ui.separator();

        if ui.button("Run").clicked() {
            self.run();
        }

        ui.separator();

        let binding = self.sync.stdout().lock().unwrap();
        let mut readonly_buffer = binding.as_str();

        ScrollArea::vertical().show(ui, |ui| {
            ui.add_sized(
                egui::vec2(PROGRAM_WIDTH, PROGRAM_HEIGHT / 3.),
                egui::TextEdit::multiline(&mut readonly_buffer),
            );
        });

        let ctx = ui.ctx();
        preview_file_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            if i.raw.dropped_files.len() == 1 {
                let item = i.raw.dropped_files[0].clone();
                if let Some(item) = item.path {
                    if item.is_file() && item.extension().is_some_and(|ext| ext == "map") {
                        self.map = item.to_str().unwrap().to_string();
                        self.use_entity = false;
                    }
                }
            }
        });

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
