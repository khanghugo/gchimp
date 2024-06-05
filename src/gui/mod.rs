use std::path::PathBuf;

use eframe::egui;
use egui_tiles::Tree;

use self::{
    config::{parse_config, Config},
    constants::{CONFIG_FILE_NAME, PROGRAM_HEIGHT, PROGRAM_WIDTH},
    pane::{create_tree, Pane, TreeBehavior},
};

mod config;
mod constants;
mod pane;
mod programs;
mod utils;

trait TabProgram {
    fn tab_title(&self) -> egui::WidgetText;
    fn tab_ui(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse;
}

pub fn gui() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // This is OKAY for now.
            .with_inner_size([PROGRAM_WIDTH, PROGRAM_HEIGHT])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::<MyApp>::default()
        }),
    )
}

struct MyApp {
    tree: Tree<Pane>,
    config: Option<Config>,
}

impl Default for MyApp {
    fn default() -> Self {
        let config_path = PathBuf::from(format!("dist/{}", CONFIG_FILE_NAME));
        let config = parse_config(config_path.display().to_string().as_str());
        let config = config.ok();

        Self {
            tree: create_tree(&config),
            config,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut behavior = TreeBehavior {};
            self.tree.ui(&mut behavior, ui);
        });
    }
}
