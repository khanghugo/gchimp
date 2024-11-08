use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui;

use crate::{
    gui::{utils::preview_file_being_dropped, TabProgram},
    modules::{loop_wave::loop_wave, split_model::split_model},
};

pub struct Misc {
    qc: String,
    wav: String,
    split_model_status: Arc<Mutex<String>>,
    loop_wave_status: Arc<Mutex<String>>,
}

impl Default for Misc {
    fn default() -> Self {
        Self {
            qc: Default::default(),
            wav: Default::default(),
            split_model_status: Arc::new(Mutex::new(String::from("Idle"))),
            loop_wave_status: Arc::new(Mutex::new(String::from("Idle"))),
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
                    #[cfg(target_arch = "x86_64")]
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
                #[cfg(target_arch = "x86_64")]
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

            if ui.button("Run").clicked() {
                self.run_loop_wave()
            }

            let binding = self.loop_wave_status.lock().unwrap();
            let mut status_text = binding.as_str();
            ui.text_edit_singleline(&mut status_text)
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
        "Running".clone_into(&mut status.lock().unwrap());

        thread::spawn(move || {
            if let Err(err) = loop_wave(wav_path) {
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

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
