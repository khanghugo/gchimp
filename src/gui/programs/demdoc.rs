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
    modules::demdoc::{
        change_map::change_map,
        check_doctored::{check_doctored, check_doctored_folder},
        kz_stats::add_kz_stats,
    },
};

pub struct DemDoc {
    bsp: String,
    dem: String,
    change_map_status: Arc<Mutex<String>>,
    kz_stats_status: Arc<Mutex<String>>,
    kz_stats_keys: bool,
    kz_stats_speedometer: bool,
    check_doctored_folder: String,
    check_doctored_use_demo: bool,
    check_doctored_use_folder: bool,
    check_doctor_status: Arc<Mutex<String>>,
    check_doctor_status_hint: Arc<Mutex<String>>,
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
            check_doctored_folder: String::new(),
            check_doctored_use_folder: true,
            check_doctored_use_demo: true,
            check_doctor_status: Arc::new(Mutex::new(String::from("Idle"))),
            check_doctor_status_hint: Arc::new(Mutex::new(String::from("Idle"))),
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

    fn ui_change_map(&mut self, ui: &mut eframe::egui::Ui) {
        ui.label("Change map")
            .on_hover_text("Changes the map of the demo");

        egui::Grid::new("Change map grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Demo:");
                ui.add(egui::TextEdit::singleline(&mut self.dem).hint_text("Choose .dem file"));
                if ui.button("Add").clicked() {
                    #[cfg(target_arch = "x86_64")]
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
                    #[cfg(target_arch = "x86_64")]
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

    fn ui_kz_stats(&mut self, ui: &mut eframe::egui::Ui) {
        ui.label("Add KZ stats")
            .on_hover_text("Adds KZ stats to demo");

        egui::Grid::new("Kz stats grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Demo:");
                ui.add(egui::TextEdit::singleline(&mut self.dem).hint_text("Choose .dem file"));
                if ui.button("Add").clicked() {
                    #[cfg(target_arch = "x86_64")]
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
    }

    fn ui_check_doctored(&mut self, ui: &mut eframe::egui::Ui) {
        ui.label("Check demdoc'd")
            .on_hover_text("Checks if demo(s) has been processed by DemDoc");

        egui::Grid::new("Check demdoc'd")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Demo:");

                if ui
                    .add_enabled(
                        self.check_doctored_use_demo,
                        egui::TextEdit::singleline(&mut self.dem).hint_text("Choose .dem file"),
                    )
                    .changed()
                {
                    self.check_doctored_use_demo = true;
                    self.check_doctored_use_folder = false;
                };

                if ui.button("Add").clicked() {
                    #[cfg(target_arch = "x86_64")]
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Demo", &["dem"])
                        .pick_file()
                    {
                        if path.extension().is_some_and(|ext| ext == "dem") {
                            self.dem = path.display().to_string();
                            self.check_doctored_use_demo = true;
                            self.check_doctored_use_folder = false;
                        }
                    }
                }
                ui.end_row();

                ui.label("Folder:");

                if ui
                    .add_enabled(
                        self.check_doctored_use_folder,
                        egui::TextEdit::singleline(&mut self.check_doctored_folder)
                            .hint_text("Choose folder"),
                    )
                    .changed()
                {
                    self.check_doctored_use_demo = false;
                    self.check_doctored_use_folder = true;
                }

                if ui.button("Add").clicked() {
                    #[cfg(target_arch = "x86_64")]
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.check_doctored_folder = path.display().to_string();
                        self.check_doctored_use_demo = false;
                        self.check_doctored_use_folder = true;
                    }
                }
                ui.end_row();

                if ui.button("Run").clicked() {
                    self.run_check_doctored()
                }

                let binding = self.check_doctor_status.lock().unwrap();
                let binding2 = self.check_doctor_status_hint.lock().unwrap();
                let mut status_text = binding.as_str();

                // click to copy
                if ui
                    .text_edit_singleline(&mut status_text)
                    .on_hover_text(binding2.as_str())
                    .clicked()
                {
                    ui.output_mut(|o| o.copied_text = binding2.to_string())
                }
            });
    }

    fn run_check_doctored(&self) {
        let dem = self.dem.clone();
        let folder = self.check_doctored_folder.clone();
        let use_folder = self.check_doctored_use_folder;
        let _use_demo = self.check_doctored_use_demo;

        let status = self.check_doctor_status.clone();
        let status_hint = self.check_doctor_status_hint.clone();

        "Running".clone_into(&mut status.lock().unwrap());
        "".clone_into(&mut status_hint.lock().unwrap());

        thread::spawn(move || {
            if dem.is_empty() && folder.is_empty() {
                return;
            }

            if use_folder {
                let res = check_doctored_folder(folder);

                if res.is_err() {
                    *status.lock().unwrap() = "Fail to read demo".to_string();
                    return;
                }

                let res = res.unwrap();

                if res.is_empty() {
                    *status.lock().unwrap() = "Found no doctored demos".to_string();
                } else {
                    *status.lock().unwrap() = format!(
                        "Found {} doctored demos. Hover or click for more",
                        res.len()
                    );
                    *status_hint.lock().unwrap() =
                        res.into_iter().fold(String::new(), |acc, path| {
                            format!("{}\n{}", acc, path.display())
                        });
                }
            } else {
                let res = check_doctored(dem);

                if res.is_err() {
                    *status.lock().unwrap() = "Fail to read demo".to_string();
                    return;
                }

                let (_, is_doctored) = res.unwrap();

                if is_doctored {
                    *status.lock().unwrap() = "Demo is doctored".to_string();
                } else {
                    *status.lock().unwrap() = "Demo is not doctored".to_string();
                }
            }
        });
    }
}

impl TabProgram for DemDoc {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "DemDoc".into()
    }

    fn tab_ui(&mut self, ui: &mut eframe::egui::Ui) -> egui_tiles::UiResponse {
        ui.separator();

        self.ui_change_map(ui);
        ui.separator();

        self.ui_kz_stats(ui);
        ui.separator();

        self.ui_check_doctored(ui);
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
                            self.check_doctored_use_demo = true;
                            self.check_doctored_use_folder = false;
                        }
                    } else if item.is_dir() {
                        self.check_doctored_folder = item.to_str().unwrap().to_string();
                        self.check_doctored_use_demo = false;
                        self.check_doctored_use_folder = true;
                    }
                }
            }
        });

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
