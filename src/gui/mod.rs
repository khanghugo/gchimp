use std::path::Path;

use eframe::egui;
use egui_tiles::Tree;
use utils::preview_file_being_dropped;

use crate::config::{parse_config, parse_config_from_file};

use self::{
    constants::{PROGRAM_HEIGHT, PROGRAM_WIDTH},
    pane::{create_tree, Pane, TreeBehavior},
};

mod constants;
mod pane;
mod programs;
mod utils;

trait TabProgram {
    fn tab_title(&self) -> egui::WidgetText {
        "MyProgram".into()
    }

    fn tab_ui(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.separator();

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}

pub fn gui() -> Result<(), eframe::Error> {
    // let icon = egui::IconData::from("../.././media/logo.png");
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../.././media/logo.png")).unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // This is OKAY for now.
            .with_inner_size([PROGRAM_WIDTH, PROGRAM_HEIGHT])
            .with_drag_and_drop(true)
            .with_icon(icon),

        ..Default::default()
    };

    eframe::run_native(
        "gchimp",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::default())
        }),
    )
}

struct MyApp {
    tree: Option<Tree<Pane>>,
    _no_config_status: String,
}

impl Default for MyApp {
    fn default() -> Self {
        let config = parse_config();

        if let Err(err) = config {
            return Self {
                tree: None,
                _no_config_status: format!("Error with parsing config.toml: {}", err),
            };
        }

        Self {
            tree: Some(create_tree(config.unwrap())),
            _no_config_status: "".to_string(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tree) = &mut self.tree {
                let mut behavior = TreeBehavior {};
                tree.ui(&mut behavior, ui);
            } else {
                if ui.button("Add config.toml").highlight().clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.parse_config(path.as_path());
                    }
                }

                let mut readonly_buffer = self._no_config_status.as_str();
                ui.add(egui::TextEdit::multiline(&mut readonly_buffer));

                let ctx = ui.ctx();

                preview_file_being_dropped(ctx);

                ctx.input(|i| {
                    for dropped_file in i.raw.dropped_files.iter() {
                        if let Some(path) = &dropped_file.path {
                            if path.extension().unwrap() == "toml" {
                                self.parse_config(path);
                            }
                        }
                    }
                });
            }
        });
    }
}

impl MyApp {
    fn parse_config(&mut self, path: &Path) {
        let config = parse_config_from_file(path);

        match config {
            Err(err) => self._no_config_status = err.to_string(),
            Ok(config) => self.tree = Some(create_tree(config)),
        }
    }
}
