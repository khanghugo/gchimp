use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui::{self, ScrollArea, Vec2};

use gchimp::modules::skymod::{SkyModBuilder, SkyModOptions};

use crate::{
    config::Config,
    gui::{
        constants::{IMAGE_FORMATS, PROGRAM_HEIGHT, PROGRAM_WIDTH},
        utils::{load_egui_image_to_texture, preview_file_being_dropped},
        TabProgram,
    },
};

static FACE_SIZE: f32 = 94.;

pub struct SkyModGui {
    app_config: Config,
    // order is: up left front right back down
    texture_paths: Vec<String>,
    texture_handles: Vec<Option<egui::TextureHandle>>,
    options: SkyModOptions,
    skybox_size: String,
    texture_per_face: String,
    idle: bool,
    program_output: Arc<Mutex<String>>,
}

impl SkyModGui {
    pub fn new(app_config: Config) -> Self {
        Self {
            app_config,
            texture_paths: vec![String::new(); 6],
            texture_handles: vec![None; 6],
            options: SkyModOptions::default(),
            skybox_size: String::from("131072"),
            texture_per_face: String::from("1"),
            idle: true,
            program_output: Arc::new(Mutex::new(String::from("Idle"))),
        }
    }

    fn run(&mut self) {
        let texture_per_face = self.texture_per_face.parse::<u32>();
        let skybox_size = self.skybox_size.parse::<u32>();

        let output = self.program_output.clone();
        "Running".clone_into(&mut output.lock().unwrap());

        if texture_per_face.is_err() {
            "Texture per face is not a number".clone_into(&mut output.lock().unwrap());
            return;
        }

        if skybox_size.is_err() {
            "Skybox size is not a number".clone_into(&mut output.lock().unwrap());
            return;
        }

        if self.options.output_name.is_empty() {
            "Output name is empty".clone_into(&mut output.lock().unwrap());
            return;
        }

        self.idle = false;

        let textures = self.texture_paths.clone();
        let studiomdl = self.app_config.studiomdl.clone();

        #[cfg(target_os = "linux")]
        let wineprefix = self.app_config.wineprefix.clone();

        let SkyModOptions {
            skybox_size: _,
            texture_per_face: _,
            convert_texture,
            flatshade,
            output_name,
        } = self.options.clone();

        let handle = thread::spawn(move || {
            let mut binding = SkyModBuilder::new();

            let skymod = binding
                .up(&textures[0])
                .lf(&textures[1])
                .ft(&textures[2])
                .rt(&textures[3])
                .bk(&textures[4])
                .dn(&textures[5])
                .skybox_size(skybox_size.unwrap())
                .texture_per_face(texture_per_face.unwrap())
                .studiomdl(studiomdl.as_str())
                .output_name(output_name.as_str())
                .convert_texture(convert_texture)
                .flat_shade(flatshade);

            #[cfg(target_os = "linux")]
            skymod.wineprefix(wineprefix);

            skymod.work()
        });

        match handle.join().unwrap() {
            Ok(_) => {
                "OK".clone_into(&mut output.lock().unwrap());
                self.idle = true;
            }
            Err(err) => {
                err.to_string()
                    .as_str()
                    .clone_into(&mut output.lock().unwrap());
                self.idle = true;
            }
        };
    }

    fn selectable_face(&mut self, ui: &mut eframe::egui::Ui, index: usize, text: &str) {
        let button = if self.texture_paths[index].is_empty() {
            ui.add_sized([FACE_SIZE, FACE_SIZE], egui::Button::new(text))
        } else {
            // now path is valid, check if the texture is loaded
            // otherwise we load new texture
            let image = egui::Image::new((
                (if let Some(handle) = &self.texture_handles[index] {
                    handle.clone()
                } else {
                    let handle =
                        load_egui_image_to_texture(ui, self.texture_paths[index].clone()).unwrap();

                    self.texture_handles[index] = Some(handle.clone());

                    handle
                })
                .id(),
                Vec2 {
                    x: FACE_SIZE,
                    y: FACE_SIZE,
                },
            ));

            ui.add_sized(
                [FACE_SIZE, FACE_SIZE],
                egui::Button::image(image).frame(false),
            )
        };

        if button.clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Image", IMAGE_FORMATS)
                .pick_file()
            {
                self.texture_paths[index] = path.display().to_string();

                // load new texture
                let handle =
                    load_egui_image_to_texture(ui, self.texture_paths[index].clone()).unwrap();

                self.texture_handles[index] = Some(handle);
            }
        };
    }
}

impl TabProgram for SkyModGui {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "SkyMod".into()
    }

    fn tab_ui(&mut self, ui: &mut eframe::egui::Ui) -> egui_tiles::UiResponse {
        ui.separator();
        ui.ctx().forget_all_images();

        egui::Grid::new("Texture grid")
            .num_columns(4)
            // .min_col_width(0.)
            // .min_row_height(0.)
            // .max_col_width(0.)
            .spacing([1., 1.])
            .show(ui, |ui| {
                ui.label("");
                if ui.button("Reset").clicked() {
                    self.texture_paths.iter_mut().for_each(|tex| tex.clear());
                    (0..self.texture_handles.len())
                        .for_each(|idx| self.texture_handles[idx] = None);
                };
                self.selectable_face(ui, 0, "U");
                ui.end_row();

                self.selectable_face(ui, 1, "L");
                self.selectable_face(ui, 4, "B");
                self.selectable_face(ui, 3, "R");
                self.selectable_face(ui, 2, "F");
                ui.end_row();

                ui.label("");
                ui.label("");
                self.selectable_face(ui, 5, "D");
                ui.end_row();
            });

        ui.separator();
        ui.label("Options:");
        ui.horizontal(|ui| {
            egui::Grid::new("option grid")
                .num_columns(4)
                .show(ui, |ui| {
                    ui.label("Texture per face:");
                    ui.text_edit_singleline(&mut self.texture_per_face)
                        .on_hover_text(
                            "\
How many textures should each skybox face have? \n
It should be a perfect square (such as 1, 4, 9, 16, ..) \n
If a model has more than 64 textures, it will be split into smaller models",
                        );
                    ui.label("Skybox size:");
                    ui.text_edit_singleline(&mut self.skybox_size)
                        .on_hover_text("The size of the model");
                });
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.options.convert_texture, "Convert texture")
                .on_hover_text(
                    "\
Converts most image format into compliant BMP. \n
Processes textures into suitable format for other settings. \n
Recommended to leave it checked.
",
                );

            ui.checkbox(&mut self.options.flatshade, "Flat shade")
                .on_hover_text(
                    "\
Mark texture with flatshade flag. \n
Recommeded to leave it checked for uniformly lit texture.",
                );

            ui.label("Output name: ");
            ui.text_edit_singleline(&mut self.options.output_name)
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(self.idle, |ui| {
                    if ui.button("Run").clicked() {
                        // TODO make it truly multithreaded like s2g
                        // kindda lazy to do it tbh
                        // it works ok enough and there isn't much on the output to report
                        self.run();
                    }
                });
                ui.add_enabled_ui(!self.idle, |ui| {
                    if ui.button("Cancel").clicked() {
                        self.idle = true;
                    }
                });
            });
        });

        ui.separator();

        let binding = self.program_output.lock().unwrap();
        let mut readonly_buffer = binding.as_str();

        ScrollArea::vertical().show(ui, |ui| {
            ui.add_sized(
                egui::vec2(PROGRAM_WIDTH, PROGRAM_HEIGHT / 16.),
                egui::TextEdit::multiline(&mut readonly_buffer),
            );
        });

        // drag and drop stuff
        let ctx = ui.ctx();
        preview_file_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            let items = i.raw.dropped_files.clone();
            let paths = items
                .iter()
                .filter_map(|e| e.path.clone())
                .collect::<Vec<PathBuf>>();

            paths.iter().for_each(|path| {
                if let Some(idx) = file_name_to_index(
                    path.file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_lowercase()
                        .as_str(),
                ) {
                    self.texture_paths[idx as usize] = path.display().to_string();

                    if let Some(handle) = &self.texture_handles[idx as usize] {
                        drop(handle.clone())
                    }

                    self.texture_handles[idx as usize] = None;

                    let skybox_name = path.file_stem().unwrap().to_str().unwrap();
                    let skybox_name = &skybox_name[..skybox_name.len() - 2];
                    self.options.output_name = skybox_name.to_string();
                }
            });
        });

        // make it continuous
        ui.ctx().request_repaint();

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}

// input is `file` or `filebk` or `fileup`
fn file_name_to_index(s: &str) -> Option<u32> {
    let last_two = &s[s.len().saturating_sub(2)..s.len()];

    // order is: up left front right back down
    let idx: u32 = match last_two {
        "up" => 0,
        "lf" => 1,
        "ft" => 2,
        "rt" => 3,
        "bk" => 4,
        "dn" => 5,
        _ => return None,
    };

    Some(idx)
}
