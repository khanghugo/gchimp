use std::io::Read;

use eframe::egui::{self, load::ImageLoader, Response};

use crate::{
    gui::{
        config::Config,
        utils::{preview_file_being_dropped, preview_files_being_dropped_min_max_file},
        TabProgram,
    },
    include_image,
};

enum FacePos {
    Front,
    Back,
    Left,
    Right,
    Up,
    Down,
}

impl FacePos {
    fn get_vec() -> Vec<Self> {
        vec![
            Self::Front,
            Self::Back,
            Self::Left,
            Self::Right,
            Self::Up,
            Self::Down,
        ]
    }
}

static FACE_SIZE: f32 = 94.;

pub struct SkyModGui {
    app_config: Option<Config>,
    // order is: up left front right back down
    textures: Vec<String>,
}

impl SkyModGui {
    pub fn new(app_config: Option<Config>) -> Self {
        Self {
            app_config,
            textures: vec![String::new(); 6],
        }
    }

    fn selectable_face(&mut self, ui: &mut eframe::egui::Ui, index: usize, text: &str) {
        let button = if self.textures[index].is_empty() {
            ui.add_sized([FACE_SIZE, FACE_SIZE], egui::Button::new(text))
        } else {
            ui.add_sized(
                [FACE_SIZE, FACE_SIZE],
                egui::ImageButton::new(include_image!(&self.textures[index])),
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
                self.selectable_face(ui, 0, "U");
                ui.end_row();

                self.selectable_face(ui, 1, "L");
                self.selectable_face(ui, 2, "F");
                self.selectable_face(ui, 3, "R");
                self.selectable_face(ui, 4, "B");
                ui.end_row();

                ui.label("");
                self.selectable_face(ui, 5, "D");
                ui.end_row();
            });

        ui.separator();
        ui.label("Options:");
        ui.horizontal(|ui| {
            ui.label("Texture per face:");
            // ui.text_edit_singleline("text")
            // ui.checkbox(&mut self.options.force, "Force")
            //     .on_hover_text("Continues with the process even when there is error.");
            // ui.checkbox(&mut self.options.add_suffix, "Add suffix")
            //     .on_hover_text("Adds suffix \"_goldsrc\" to the name of the converted model");
            // ui.checkbox(&mut self.options.ignore_converted, "Ignore converted")
            //     .on_hover_text("Ignores models with \"_goldsrc\" suffix");
        });
        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
