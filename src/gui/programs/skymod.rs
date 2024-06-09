use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui::{self, ScrollArea};

use crate::{
    config::Config,
    gui::{
        constants::{PROGRAM_HEIGHT, PROGRAM_WIDTH},
        utils::preview_file_being_dropped,
        TabProgram,
    },
    include_image,
    modules::skymod::{SkyModBuilder, SkyModOptions},
};

static FACE_SIZE: f32 = 94.;
static SKY_TEXTURE_SUFFIX: [&str; 6] = ["up", "dn", "lf", "rt", "ft", "bk"];

pub struct SkyModGui {
    app_config: Config,
    // order is: up left front right back down
    textures: Vec<String>,
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
            textures: vec![String::new(); 6],
            options: SkyModOptions::default(),
            skybox_size: String::from("131072"),
            texture_per_face: String::from("1"),
            idle: true,
            program_output: Arc::new(Mutex::new(String::from("Idle"))),
        }
    }

    fn program_output(&mut self, s: &str) {
        let mut lock = self.program_output.lock().unwrap();
        *lock = s.to_string();
    }

    fn run(&mut self) {
        let texture_per_face = self.texture_per_face.parse::<u32>();
        let skybox_size = self.skybox_size.parse::<u32>();

        if texture_per_face.is_err() {
            self.program_output("Texture per face is not a number");
            return;
        }

        if skybox_size.is_err() {
            self.program_output("Skybox size is not a number");
            return;
        }

        if self.options.output_name.is_empty() {
            self.program_output("Output name is empty");
            return;
        }

        self.idle = false;

        let textures = self.textures.clone();
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
                self.program_output("OK");
                self.idle = true;
            }
            Err(err) => {
                self.program_output(err.to_string().as_str());
                self.idle = true;
            }
        };
    }

    fn selectable_face(&mut self, ui: &mut eframe::egui::Ui, index: usize, text: &str) {
        let button = if self.textures[index].is_empty() {
            ui.add_sized([FACE_SIZE, FACE_SIZE], egui::Button::new(text))
        } else {
            ui.add_sized(
                [FACE_SIZE, FACE_SIZE],
                egui::ImageButton::new(include_image!(&self.textures[index])).frame(false),
            )
        };

        if button.clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                self.textures[index] = path.display().to_string();
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

        egui::Grid::new("Texture grid")
            .num_columns(4)
            // .min_col_width(0.)
            // .min_row_height(0.)
            // .max_col_width(0.)
            .spacing([1., 1.])
            .show(ui, |ui| {
                ui.label("");
                if ui.button("Reset").clicked() {
                    self.textures.iter_mut().for_each(|tex| tex.clear());
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
It should be a perfect square (such as 2, 4, 9, 16, ..) \n
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
            if i.raw.dropped_files.len() == 6 {
                let items = i.raw.dropped_files.clone();
                let paths = items
                    .iter()
                    .filter_map(|e| e.path.clone())
                    .collect::<Vec<PathBuf>>();

                if paths.len() != 6 {
                    return;
                }

                let is_valid = paths.iter().all(|p| {
                    let a = p.file_stem().unwrap().to_str().unwrap().to_lowercase();
                    SKY_TEXTURE_SUFFIX.iter().any(|suffix| a.ends_with(suffix))
                });

                if !is_valid {
                    return;
                }

                self.options.output_name = paths[0]
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace("bk", "");

                self.textures[0] = paths
                    .iter()
                    .find(|p| {
                        p.file_stem()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_lowercase()
                            .ends_with("up")
                    })
                    .unwrap()
                    .display()
                    .to_string();
                self.textures[1] = paths
                    .iter()
                    .find(|p| {
                        p.file_stem()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_lowercase()
                            .ends_with("lf")
                    })
                    .unwrap()
                    .display()
                    .to_string();
                self.textures[2] = paths
                    .iter()
                    .find(|p| {
                        p.file_stem()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_lowercase()
                            .ends_with("ft")
                    })
                    .unwrap()
                    .display()
                    .to_string();
                self.textures[3] = paths
                    .iter()
                    .find(|p| {
                        p.file_stem()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_lowercase()
                            .ends_with("rt")
                    })
                    .unwrap()
                    .display()
                    .to_string();
                self.textures[4] = paths
                    .iter()
                    .find(|p| {
                        p.file_stem()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_lowercase()
                            .ends_with("bk")
                    })
                    .unwrap()
                    .display()
                    .to_string();
                self.textures[5] = paths
                    .iter()
                    .find(|p| {
                        p.file_stem()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_lowercase()
                            .ends_with("dn")
                    })
                    .unwrap()
                    .display()
                    .to_string();
            }
        });

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
