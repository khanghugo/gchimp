use std::{
    path::PathBuf,
    thread::{self, JoinHandle},
};

use eframe::egui::{self, ScrollArea};

use crate::{
    gui::{
        config::Config,
        constants::{PROGRAM_HEIGHT, PROGRAM_WIDTH},
        utils::preview_files_being_dropped,
        TabProgram,
    },
    modules::s2g::{S2GBuilder, S2GOptions, S2GSteps, S2GSync},
};

struct DragAndDrop {
    file_path: String,
    use_file: bool,
    folder_path: String,
    use_folder: bool,
}

impl Default for DragAndDrop {
    fn default() -> Self {
        // use_file and use_path are both true so the users can choose either at the beginning.
        Self {
            file_path: Default::default(),
            use_file: true,
            folder_path: Default::default(),
            use_folder: true,
        }
    }
}

pub struct S2GGui {
    app_config: Option<Config>,
    s2g_sync: S2GSync,
    drag_and_drop: DragAndDrop,
    steps: S2GSteps,
    options: S2GOptions,
    is_idle: bool,
}

impl S2GGui {
    // runs in a different thread to avoid blocking
    fn run(&self) -> JoinHandle<eyre::Result<Vec<PathBuf>>> {
        let path = self.drag_and_drop.file_path.clone();
        let steps = self.steps.clone();
        let options = self.options.clone();

        let wineprefix = if let Some(app_config) = &self.app_config {
            app_config.wineprefix.clone()
        } else {
            None
        };

        let sync = self.s2g_sync.clone();

        let handle = thread::spawn(move || {
            *sync.is_done().lock().unwrap() = false;

            let mut s2g = S2GBuilder::new_with_path_to_bin(path.as_str(), "dist");

            let S2GSteps {
                decompile,
                vtf,
                bmp,
                smd_and_qc,
                compile,
            } = steps;

            let S2GOptions { force, add_suffix } = options;

            s2g.decompile(decompile)
                .vtf(vtf)
                .bmp(bmp)
                .smd_and_qc(smd_and_qc)
                .compile(compile)
                .sync(sync.clone())
                .force(force)
                .add_suffix(add_suffix);

            #[cfg(target_os = "linux")]
            if let Some(wineprefix) = wineprefix {
                s2g.set_wineprefix(wineprefix.as_str());
            }

            let res = s2g.work();

            *sync.is_done().lock().unwrap() = true;

            res
        });

        handle
    }

    pub fn new(app_config: Option<Config>) -> Self {
        Self {
            app_config,
            s2g_sync: S2GSync::default(),
            drag_and_drop: DragAndDrop::default(),
            steps: S2GSteps::default(),
            options: S2GOptions::default(),
            is_idle: true,
        }
    }
}

impl TabProgram for S2GGui {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "S2G".into()
    }

    fn tab_ui(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.separator();

        ui.add_enabled_ui(true, |ui| {
            egui::Grid::new("S2G Layout").num_columns(2).show(ui, |ui| {
                ui.label("File:");
                ui.add_enabled_ui(self.drag_and_drop.use_file, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.drag_and_drop.file_path)
                            .hint_text("Choose .mdl file"),
                    );
                });
                if ui.button("+").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.drag_and_drop.file_path = path.display().to_string();
                        self.drag_and_drop.use_file = true;
                        self.drag_and_drop.use_folder = false;
                    }
                }
                ui.end_row();

                ui.label("Folder:");
                ui.add_enabled_ui(self.drag_and_drop.use_folder, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.drag_and_drop.folder_path)
                            .hint_text("Choose folder containing .mdl"),
                    );
                });
                if ui.button("+").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.drag_and_drop.folder_path = path.display().to_string();
                        self.drag_and_drop.use_folder = true;
                        self.drag_and_drop.use_file = false;
                    }
                }
            })
        });

        // if compile is ticked then always do the smd and qc step
        if self.steps.compile {
            self.steps.smd_and_qc = true;
        }

        ui.separator();
        ui.label("Steps:");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.steps.decompile, "Decompile");
            ui.checkbox(&mut self.steps.vtf, "VTF")
                .on_hover_text("Uses no_vtf to convert all .vtx files in the folder to .png");
            ui.checkbox(&mut self.steps.bmp, "BMP")
                .on_hover_text("Converts all .png in the folder to compliant .bmp");
            ui.checkbox(&mut self.steps.smd_and_qc, "Smd/Qc")
                .on_hover_text("Converts decompiled Smd/Qc files");
            ui.checkbox(&mut self.steps.compile, "GoldSrc Compile")
                .on_hover_text("Must have Smd/Qc step enabled");
        });

        ui.separator();
        ui.label("Options:");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.options.force, "Force")
                .on_hover_text("Continues with the process even when there is error.");
            ui.checkbox(&mut self.options.add_suffix, "Add suffix")
                .on_hover_text("Adds suffix \"_goldsrc\" to the name of the converted model");
        });

        let is_done = *self.s2g_sync.is_done().lock().unwrap();

        ui.separator();
        ui.horizontal(|ui| {
            ui.add_enabled_ui(is_done, |ui| {
                if ui.button("Run").clicked() {
                    self.is_idle = false;
                    self.run();
                }
            });
            ui.add_enabled_ui(!is_done, |ui| {
                if ui.button("Cancel").clicked() {
                    self.is_idle = true;
                }
            });
        });

        ui.separator();

        let binding = self.s2g_sync.stdout().lock().unwrap();
        let mut readonly_buffer = binding.as_str();

        ScrollArea::vertical().show(ui, |ui| {
            ui.add_sized(
                egui::vec2(PROGRAM_WIDTH, PROGRAM_HEIGHT / 3.),
                // Unironically the way to make textbox immutable, LMFAO
                egui::TextEdit::multiline(&mut readonly_buffer),
                // egui::TextEdit::multiline(&mut self.output.as_str()),
            );
        });

        let ctx = ui.ctx();
        preview_files_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            if i.raw.dropped_files.len() == 1 {
                let item = i.raw.dropped_files[0].clone();
                if let Some(item) = item.path {
                    if item.is_file() {
                        self.drag_and_drop.file_path = item.display().to_string();
                        self.drag_and_drop.use_file = true;
                        self.drag_and_drop.use_folder = false;
                    } else if item.is_dir() {
                        self.drag_and_drop.folder_path = item.display().to_string();
                        self.drag_and_drop.use_folder = true;
                        self.drag_and_drop.use_file = false;
                    }
                } else {
                    todo!("Do something about file not being recognizable or just don't")
                }
            }
        });

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
