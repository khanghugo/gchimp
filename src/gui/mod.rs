use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use eframe::egui::{self, ThemePreference};
use egui_tiles::Tree;
use utils::preview_file_being_dropped;

use crate::{
    config::{parse_config, parse_config_from_file, Config},
    err,
    persistent_storage::PersistentStorage,
};

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

#[cfg(target_arch = "x86_64")]
pub fn gui() -> eyre::Result<()> {
    let config_res = parse_config();

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../.././media/logo.png")).unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // This is OKAY for now.
            .with_inner_size([PROGRAM_WIDTH, PROGRAM_HEIGHT])
            .with_drag_and_drop(true)
            .with_icon(icon)
            .with_maximize_button(false)
            .with_minimize_button(false),
        ..Default::default()
    };

    let theme_preference = if let Ok(config) = &config_res {
        if config.theme.contains("light") {
            ThemePreference::Light
        } else if config.theme.contains("dark") {
            ThemePreference::Dark
        } else {
            ThemePreference::System
        }
    } else {
        ThemePreference::System
    };

    let persistent_storage = PersistentStorage::start()?;
    let persistent_storage = Arc::new(Mutex::new(persistent_storage));

    let gui_res = eframe::run_native(
        "gchimp",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp::new(
                config_res,
                persistent_storage,
                theme_preference,
            )))
        }),
    );

    match gui_res {
        Ok(_) => Ok(()),
        Err(err) => err!("Error with running gchimp GUI: {}", err),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn gui() -> eyre::Result<()> {
    todo!("gui wasm32")
}

pub struct MyApp {
    tree: Option<Tree<Pane>>,
    _no_config_status: String,
    // duplicated because create_tree should have been a struct method
    persistent_storage: Arc<Mutex<PersistentStorage>>,
    theme: ThemePreference,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.options_mut(|option| option.theme_preference = self.theme);

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tree) = &mut self.tree {
                let mut behavior = TreeBehavior {};
                tree.ui(&mut behavior, ui);
            } else {
                if ui.button("Add config.toml").highlight().clicked() {
                    #[cfg(target_arch = "x86_64")]
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
    pub fn new(
        config_res: eyre::Result<Config>,
        persistent_storage: Arc<Mutex<PersistentStorage>>,
        theme: ThemePreference,
    ) -> Self {
        if let Err(err) = config_res {
            return Self {
                tree: None,
                _no_config_status: format!("Error with parsing config.toml: {}", err),
                persistent_storage,
                theme,
            };
        }

        Self {
            tree: Some(create_tree(config_res.unwrap(), persistent_storage.clone())),
            _no_config_status: "".to_string(),
            persistent_storage,
            theme,
        }
    }

    fn parse_config(&mut self, path: &Path) {
        let config = parse_config_from_file(path);

        match config {
            Err(err) => self._no_config_status = err.to_string(),
            Ok(config) => self.tree = Some(create_tree(config, self.persistent_storage.clone())),
        }
    }
}
