use std::{
    env,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui::{self, ScrollArea};

use gchimp::modules::map2mdl::{
    convert_all_map2mdl_entities, convert_entire_map, convert_world_brush_entity,
    entity::MAP2MDL_ENTITY_NAME,
    types::{Map2MdlEntitySpawnflag, Map2MdlOption},
};

use crate::{
    config::Config,
    gui::{
        TabProgram,
        constants::{PROGRAM_HEIGHT, PROGRAM_WIDTH},
        utils::preview_file_being_dropped,
    },
};

struct ExtraOptions {
    convert_only_marked_entity: bool,
    center_model: bool,
}

impl Default for ExtraOptions {
    fn default() -> Self {
        Self {
            convert_only_marked_entity: false,
            center_model: true,
        }
    }
}

pub struct Map2MdlGui {
    #[allow(unused)]
    app_config: Config,
    map: String,
    entity: String,
    use_entity: bool,
    options: Map2MdlOption,
    extra_options: ExtraOptions,
    status_text: Arc<Mutex<String>>,
}

impl Map2MdlGui {
    pub fn new(app_config: Config) -> Self {
        Self {
            app_config,
            map: Default::default(),
            entity: Default::default(),
            use_entity: false,
            options: Map2MdlOption::default(),
            extra_options: ExtraOptions::default(),
            status_text: Default::default(),
        }
    }

    fn run(&mut self) {
        {
            *self.status_text.lock().unwrap() = "Running".to_string();
        }

        let entity_text = self.entity.clone();
        let map_path = self.map.clone();
        let use_entity = self.use_entity;
        let convert_only_marked_entity = self.extra_options.convert_only_marked_entity;

        const ENTITY_TEXT_MODEL_FILE_NAME: &str = "map2mdl.mdl";
        let current_exe =
            env::current_exe().expect("cannot get currently running gchimp executable path`");

        let sync = self.status_text.clone();

        let mut entity_option = self.options.clone();

        thread::spawn(move || {
            let res = if use_entity {
                entity_option.output = current_exe.with_file_name(ENTITY_TEXT_MODEL_FILE_NAME);

                convert_world_brush_entity(&entity_text, &entity_option)
            } else {
                if convert_only_marked_entity {
                    convert_all_map2mdl_entities(map_path)
                } else {
                    entity_option.output = Path::new(&map_path).to_path_buf();

                    convert_entire_map(map_path, &entity_option)
                }
            };

            if let Err(err) = res {
                let mut lock = sync.lock().unwrap();
                *lock = err.to_string();
            } else {
                let mut ok_text = "OK".to_string();

                if use_entity {
                    ok_text = format!(
                        "{ok_text}\nModel is saved at {}",
                        current_exe
                            .with_file_name(ENTITY_TEXT_MODEL_FILE_NAME)
                            .display()
                    );
                }

                *sync.lock().unwrap() = ok_text;
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
                    if ui.button("Add").clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter("MAP", &["map"])
                            .pick_file()
                        && path.extension().is_some_and(|ext| ext == "map")
                    {
                        self.map = path.display().to_string();
                        self.use_entity = false;
                    }

                    ui.end_row();
                    ui.checkbox(&mut self.use_entity, "Entity");
                    ui.add_enabled_ui(self.use_entity, |ui| {
                        egui::ScrollArea::vertical()
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.entity)
                                        .hint_text("Worldbrush entity copied from TrechBroom")
                                        .desired_rows(1)
                                        .cursor_at_end(true),
                                );
                            });
                    });
                    if ui.button("Clear").clicked() {
                        self.entity.clear();
                    }
                })
        });
        ui.separator();
        ui.label("Options:");

        ui.horizontal(|ui| {
            ui.checkbox(
                &mut self.extra_options.convert_only_marked_entity,
                "Only convert marked entity",
            )
            .on_hover_text(format!(
                "Only convert brush entities {} and this would modify the original map file",
                MAP2MDL_ENTITY_NAME
            ));
            ui.checkbox(&mut self.extra_options.center_model, "Center the model")
                .on_hover_text("Model origin is at center of its volume");

            {
                let mut flatshade = self
                    .options
                    .spawnflags
                    .contains(Map2MdlEntitySpawnflag::FlatShade);

                if ui
                    .checkbox(&mut flatshade, "Flatshade")
                    .on_hover_text("Model is flatshade")
                    .changed()
                {
                    self.options
                        .spawnflags
                        .set(Map2MdlEntitySpawnflag::FlatShade, flatshade);
                }
            }
        });

        ui.horizontal(|ui| {
            let mut reverse_normal = self
                .options
                .spawnflags
                .contains(Map2MdlEntitySpawnflag::ReverseNormals);
            let mut with_cel_shade = self
                .options
                .spawnflags
                .contains(Map2MdlEntitySpawnflag::WithCelShade);

            if ui
                .checkbox(&mut reverse_normal, "Reverse normal")
                .on_hover_text("Reverses every vertex normals")
                .changed()
            {
                self.options
                    .spawnflags
                    .set(Map2MdlEntitySpawnflag::ReverseNormals, reverse_normal);
            }

            if ui
                .checkbox(&mut with_cel_shade, "CelShade")
                .on_hover_text("Enable cel shading")
                .changed()
            {
                self.options
                    .spawnflags
                    .set(Map2MdlEntitySpawnflag::WithCelShade, with_cel_shade);
            }

            ui.add_enabled_ui(with_cel_shade, |ui| {
                // let color_picker = egui::color_picker::color_picker_color32(ui, srgba, alpha)
                ui.label("Color");
                ui.color_edit_button_srgb(&mut self.options.celshade_options.color);

                ui.label("Distance");
                let drag = egui::DragValue::new(&mut self.options.celshade_options.distance)
                    .range(0.0..=128.0);
                ui.add(drag);
            });
        });

        ui.separator();

        if ui.button("Run").clicked() {
            self.run();
        }

        ui.separator();

        let binding = self.status_text.lock().unwrap();
        let mut readonly_buffer = binding.as_str();

        ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
            ui.add_sized(
                egui::vec2(PROGRAM_WIDTH, PROGRAM_HEIGHT / 3.),
                egui::TextEdit::multiline(&mut readonly_buffer).cursor_at_end(true),
            );
        });

        let ctx = ui.ctx();
        preview_file_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            if i.raw.dropped_files.len() == 1 {
                let item = i.raw.dropped_files[0].clone();
                if let Some(item) = item.path
                    && item.is_file()
                    && item.extension().is_some_and(|ext| ext == "map")
                {
                    self.map = item.to_str().unwrap().to_string();
                    self.use_entity = false;
                }
            }
        });

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
