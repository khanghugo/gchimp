use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui;

use gchimp::modules::{
    loop_wave::loop_wave,
    resmake::{ResMake, ResMakeOptions},
    split_model::split_model,
};

use crate::gui::{utils::preview_file_being_dropped, TabProgram};

pub struct Misc {
    qc: String,
    wav: String,
    bsp: String,
    resmake_options: ResMakeOptions,
    split_model_status: Arc<Mutex<String>>,
    loop_wave_loop: bool,
    loop_wave_status: Arc<Mutex<String>>,
    resmake_status: Arc<Mutex<String>>,
}

impl Default for Misc {
    fn default() -> Self {
        Self {
            qc: Default::default(),
            wav: Default::default(),
            bsp: Default::default(),
            resmake_options: Default::default(),
            split_model_status: Arc::new(Mutex::new(String::from("Idle"))),
            loop_wave_status: Arc::new(Mutex::new(String::from("Idle"))),
            resmake_status: Arc::new(Mutex::new(String::from("Idle"))),
            loop_wave_loop: true,
        }
    }
}

impl Misc {
    fn split_model(&mut self, ui: &mut eframe::egui::Ui) {
        ui.label("Split model").on_hover_text(
            "Splits a complete .qc linked with ONE smd to produce more smds with less triangles",
        );
        egui::Grid::new("split_model")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("QC:");
                ui.add(egui::TextEdit::singleline(&mut self.qc).hint_text("Choose .qc file"));
                if ui.button("Add").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("QC", &["qc"]).pick_file()
                    {
                        if path.extension().is_some_and(|ext| ext == "qc") {
                            self.qc = path.display().to_string();
                        }
                    }
                }
                ui.end_row();

                if ui.button("Run").clicked() {
                    self.run_split_model();
                }

                let binding = self.split_model_status.lock().unwrap();
                let mut status_text = binding.as_str();
                ui.text_edit_singleline(&mut status_text)
            });
    }

    fn loop_wave(&mut self, ui: &mut eframe::egui::Ui) {
        ui.label("Loop wave")
            .on_hover_text("Makes a .wav file loop");
        egui::Grid::new("loop_wav").num_columns(2).show(ui, |ui| {
            ui.label("WAV:");
            ui.add(egui::TextEdit::singleline(&mut self.wav).hint_text("Choose .wav file"));
            if ui.button("Add").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("WAV", &["wav"])
                    .pick_file()
                {
                    if path.extension().is_some_and(|ext| ext == "wav") {
                        self.wav = path.display().to_string();
                    }
                }
            }
            ui.end_row();

            ui.checkbox(&mut self.loop_wave_loop, "Loop")
                .on_hover_text("Loop the wave");

            ui.end_row();

            if ui.button("Run").clicked() {
                self.run_loop_wave()
            }

            let binding = self.loop_wave_status.lock().unwrap();
            let mut status_text = binding.as_str();
            ui.text_edit_singleline(&mut status_text)
        });
    }

    fn resmake(&mut self, ui: &mut eframe::egui::Ui) {
        ui.label("ResMake")
            .on_hover_text("Basically a RESGEN clone");
        egui::Grid::new("resmake").num_columns(2).show(ui, |ui| {
            ui.label("BSP:");
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
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.resmake_options.res, "RES")
                .on_hover_text("Creates RES");
            ui.checkbox(&mut self.resmake_options.zip, "ZIP")
                .on_hover_text("Creates ZIP");
            ui.checkbox(&mut self.resmake_options.wad_check, "WAD")
                .on_hover_text(
                    "\
Whether to include external WAD in .res if found.
Should only be used when BSP file is inside a proper folder structure.",
                );

            ui.checkbox(
                &mut self.resmake_options.include_default_resource,
                "Default res",
            )
            .on_hover_text(
                "\
Whether to include resource files from base game.",
            );

            ui.checkbox(
                &mut self.resmake_options.zip_ignore_missing,
                "Ignore missing",
            )
            .on_hover_text(
                "\
Ignore errors when encounter missing resource.",
            );
        });

        ui.horizontal(|ui| {
            ui.checkbox(
                &mut self.resmake_options.create_linked_wad,
                "Create linked WAD",
            )
            .on_hover_text(
                "\
If there are external WADs found, this option will create a new WAD file containing only used texture derived from those WAD files.",
            );
        });

        ui.horizontal(|ui| {
            if ui.button("Run").clicked() {
                self.run_resmake()
            }

            let binding = self.resmake_status.lock().unwrap();
            let mut status_text = binding.as_str();
            ui.text_edit_singleline(&mut status_text);
        });
    }

    fn run_split_model(&mut self) {
        let qc = self.qc.clone();
        let status = self.split_model_status.clone();
        "Running".clone_into(&mut status.lock().unwrap());

        thread::spawn(move || {
            if let Err(err) = split_model(qc.as_str()) {
                err.to_string().clone_into(&mut status.lock().unwrap());
            } else {
                "Done".clone_into(&mut status.lock().unwrap());
            }
        });
    }

    fn run_loop_wave(&mut self) {
        let wav = self.wav.clone();
        let wav_path = PathBuf::from(wav);
        let status = self.loop_wave_status.clone();
        let loop_ = self.loop_wave_loop;
        "Running".clone_into(&mut status.lock().unwrap());

        thread::spawn(move || {
            if let Err(err) = loop_wave(wav_path, loop_) {
                err.to_string().clone_into(&mut status.lock().unwrap());
            } else {
                "Done".clone_into(&mut status.lock().unwrap());
            }
        });
    }

    fn run_resmake(&mut self) {
        let bsp = self.bsp.clone();
        let bsp_path = PathBuf::from(bsp);
        let status = self.resmake_status.clone();
        let ResMakeOptions {
            wad_check,
            include_default_resource,
            res,
            zip,
            zip_ignore_missing,
            create_linked_wad,
            skip_created_res: _,
        } = self.resmake_options;
        "Running".clone_into(&mut status.lock().unwrap());

        thread::spawn(move || {
            let mut resmake = ResMake::new();

            resmake
                .wad_check(wad_check)
                .include_default_resource(include_default_resource)
                .res(res)
                .zip(zip)
                .zip_ignore_missing(zip_ignore_missing)
                .create_linked_wad(create_linked_wad);

            resmake.bsp_file(bsp_path);

            if let Err(err) = resmake.run() {
                err.to_string().clone_into(&mut status.lock().unwrap());
            } else {
                "Done".clone_into(&mut status.lock().unwrap());
            }
        });
    }
}

impl TabProgram for Misc {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "Misc".into()
    }

    fn tab_ui(&mut self, ui: &mut eframe::egui::Ui) -> egui_tiles::UiResponse {
        ui.separator();

        self.split_model(ui);
        ui.separator();

        self.loop_wave(ui);
        ui.separator();

        self.resmake(ui);
        ui.separator();

        let ctx = ui.ctx();
        preview_file_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            for item in i.raw.dropped_files.clone() {
                if let Some(item) = item.path {
                    if item.is_file() {
                        if item.extension().is_some_and(|ext| ext == "qc") {
                            self.qc = item.to_str().unwrap().to_string();
                        } else if item.extension().is_some_and(|ext| ext == "wav") {
                            self.wav = item.to_str().unwrap().to_string();
                        }
                    }
                }
            }
        });

        // runs in continuous mode
        ctx.request_repaint();

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
