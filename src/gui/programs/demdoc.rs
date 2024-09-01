use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use bsp::Bsp;
use dem::{open_demo, write_demo};
use eframe::egui;

use crate::{
    gui::{utils::preview_file_being_dropped, TabProgram},
    modules::demdoc::{change_map::change_map, kz_stats::add_kz_stats},
};

pub struct DemDoc {
    bsp: String,
    dem: String,
    change_map_status: Arc<Mutex<String>>,
    kz_stats_status: Arc<Mutex<String>>,
    kz_stats_keys: bool,
    kz_stats_speedometer: bool,
}

impl Default for DemDoc {
    fn default() -> Self {
        Self {
            bsp: String::new(),
            dem: String::new(),
            change_map_status: Arc::new(Mutex::new(String::from("Idle"))),
            kz_stats_status: Arc::new(Mutex::new(String::from("Idle"))),
            kz_stats_keys: true,
            kz_stats_speedometer: true,
        }
    }
}

impl DemDoc {
    fn run_change_map(&self) {
        let bsp = self.bsp.clone();
        let dem = self.dem.clone();
        let status = self.change_map_status.clone();

        thread::spawn(move || {
            let mut status = status.lock().unwrap();

            "Running".clone_into(&mut status);

            let bsp_path = PathBuf::from(bsp);
            let new_bsp_name = bsp_path.file_name().unwrap().to_str().unwrap().to_owned();
            let bsp = Bsp::from_file(&bsp_path);

            if let Err(err) = &bsp {
                *status = format!("Cannot open .bsp: {}", err);
            }

            let bsp = bsp.unwrap();
            let what = dem.clone();
            let mut demo = open_demo(what).unwrap();

            change_map(&mut demo, &bsp, new_bsp_name.as_str());

            let out_path = format!("{}_demdoc.dem", dem.strip_suffix(".dem").unwrap());
            let what = out_path.clone();

            if let Err(err) = write_demo(what, demo) {
                *status = format!("Cannot write .dem: {}", err);
            }

            format!(
                "File written at ..{}",
                &out_path[out_path.len().saturating_sub(32)..]
            )
            .clone_into(&mut status);
        });
    }

    fn run_kz_stats(&self) {
        let dem = self.dem.clone();
        let status = self.kz_stats_status.clone();

        let add_keys = self.kz_stats_keys;
        let add_speedometer = self.kz_stats_speedometer;

        thread::spawn(move || {
            let mut status = status.lock().unwrap();

            "Running".clone_into(&mut status);

            let what = dem.clone();
            let mut demo = open_demo(what).unwrap();

            add_kz_stats(&mut demo, |addons| {
                if add_keys {
                    addons.add_keys();
                }

                if add_speedometer {
                    addons.add_speedometer();
                }
            });

            let out_path = format!("{}_demdoc.dem", dem.strip_suffix(".dem").unwrap());
            let what = out_path.clone();

            if let Err(err) = write_demo(what, demo) {
                *status = format!("Cannot write .dem: {}", err);
            }

            format!(
                "File written at ..{}",
                &out_path[out_path.len().saturating_sub(24)..]
            )
            .clone_into(&mut status);
        });
    }
}

impl TabProgram for DemDoc {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "DemDoc".into()
    }

    fn tab_ui(&mut self, ui: &mut eframe::egui::Ui) -> egui_tiles::UiResponse {
        ui.separator();
        ui.label("Change map")
            .on_hover_text("Changes the map of the demo");

        egui::Grid::new("Change map grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Demo:");
                ui.add(egui::TextEdit::singleline(&mut self.dem).hint_text("Choose .dem file"));
                if ui.button("Add").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Demo", &["dem"])
                        .pick_file()
                    {
                        if path.extension().is_some_and(|ext| ext == "dem") {
                            self.dem = path.display().to_string();
                        }
                    }
                }
                ui.end_row();

                ui.label("Map:");
                ui.add(egui::TextEdit::singleline(&mut self.bsp).hint_text("Choose .bsp file"));
                if ui.button("Add").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("BSP", &["bsp"])
                        .pick_file()
                    {
                        if path.extension().is_some_and(|ext| ext == "bsp") {
                            self.bsp = path.display().to_string();
                        }
                    }
                }
                ui.end_row();

                if ui.button("Run").clicked() {
                    self.run_change_map()
                }

                let binding = self.change_map_status.lock().unwrap();
                let mut status_text = binding.as_str();
                ui.text_edit_singleline(&mut status_text)
            });
        ui.separator();

        ui.label("Add KZ stats")
            .on_hover_text("Adds KZ stats to demo");

        egui::Grid::new("Kz stats grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Demo:");
                ui.add(egui::TextEdit::singleline(&mut self.dem).hint_text("Choose .dem file"));
                if ui.button("Add").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Demo", &["dem"])
                        .pick_file()
                    {
                        if path.extension().is_some_and(|ext| ext == "dem") {
                            self.dem = path.display().to_string();
                        }
                    }
                }
                ui.end_row();
            });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.kz_stats_keys, "Keys");
            ui.checkbox(&mut self.kz_stats_speedometer, "Speed");

            if ui.button("Run").clicked() {
                self.run_kz_stats();
            }

            let binding = self.kz_stats_status.lock().unwrap();
            let mut status_text = binding.as_str();
            ui.text_edit_singleline(&mut status_text);
        });

        ui.separator();

        let ctx = ui.ctx();
        preview_file_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            for item in i.raw.dropped_files.clone() {
                if let Some(item) = item.path {
                    if item.is_file() {
                        if item.extension().is_some_and(|ext| ext == "bsp") {
                            self.bsp = item.to_str().unwrap().to_string();
                        } else if item.extension().is_some_and(|ext| ext == "dem") {
                            self.dem = item.to_str().unwrap().to_string();
                        }
                    }
                }
            }
        });

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
