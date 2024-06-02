use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
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
    modules::s2g::{S2GOptions, S2GSync},
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

#[derive(Clone)]
struct Steps {
    decompile: bool,
    vtf: bool,
    smd_and_qc: bool,
    goldsrc_compile: bool,
}

impl Default for Steps {
    fn default() -> Self {
        Self {
            decompile: true,
            vtf: true,
            smd_and_qc: true,
            goldsrc_compile: true,
        }
    }
}

struct Options {
    force: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { force: false }
    }
}

pub struct S2GGui {
    app_config: Option<Config>,
    s2g: Option<S2GOptions>,
    s2g_sync: S2GSync,
    drag_and_drop: DragAndDrop,
    steps: Steps,
    options: Options,
    is_idle: bool,
}

impl S2GGui {
    // runs in a different thread to avoid blocking
    fn run(&mut self) -> JoinHandle<eyre::Result<Vec<PathBuf>>> {
        let path = self.drag_and_drop.file_path.clone();
        let steps = self.steps.clone();

        let wineprefix = if let Some(app_config) = &self.app_config {
            app_config.wineprefix.clone()
        } else {
            None
        };

        let sync = self.s2g_sync.clone();

        let handle = thread::spawn(move || {
            let mut s2g = S2GOptions::new_with_path_to_bin(path.as_str(), "dist");

            let Steps {
                decompile,
                vtf,
                smd_and_qc,
                goldsrc_compile,
            } = steps;

            s2g.decompile(decompile)
                .vtf(vtf)
                .smd_and_qc(smd_and_qc)
                .compile(goldsrc_compile)
                .sync(sync)
                // .force(force)
                ;

            #[cfg(target_os = "linux")]
            if let Some(wineprefix) = wineprefix {
                s2g.set_wineprefix(wineprefix.as_str());
            }

            s2g.work()
            // let _ = self.s2g.as_mut().unwrap().work();
        });

        handle
        // self.s2g = Some(S2GOptions::new_with_path_to_bin(
        //     &self.drag_and_drop.file_path,
        //     "dist",
        // ));

        // let Steps {
        //     decompile,
        //     vtf,
        //     smd_and_qc,
        //     goldsrc_compile,
        // } = self.steps;

        // self.s2g.as_mut().unwrap().decompile(decompile)
        //     .vtf(vtf)
        //     .smd_and_qc(smd_and_qc)
        //     .compile(goldsrc_compile)
        //     // .force(force)
        //     ;

        // // #[cfg(target_os = "linux")]
        // self.s2g.as_mut().unwrap().set_wine_prefix(
        //     self.app_config
        //         .as_ref()
        //         .unwrap()
        //         .wineprefix
        //         .as_ref()
        //         .unwrap()
        //         .as_str(),
        // );

        // let _ = self.s2g.as_mut().unwrap().work();
    }

    pub fn new(app_config: Option<Config>) -> Self {
        Self {
            app_config,
            s2g: None,
            s2g_sync: S2GSync::default(),
            drag_and_drop: DragAndDrop::default(),
            steps: Steps::default(),
            options: Options::default(),
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
                            .hint_text("Choose .mdl file (or .qc)"),
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

        ui.separator();
        ui.label("Steps:");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.steps.decompile, "Decompile");
            ui.checkbox(&mut self.steps.vtf, "VTF");
            ui.checkbox(&mut self.steps.smd_and_qc, "Smd/Qc");
            ui.checkbox(&mut self.steps.goldsrc_compile, "GoldSrc Compile");
        });

        ui.separator();
        ui.label("Options:");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.options.force, "Force")
                .on_hover_text("Continue with the process even when there is error.");
        });

        ui.separator();
        ui.horizontal(|ui| {
            ui.add_enabled_ui(self.is_idle, |ui| {
                if ui.button("Run").clicked() {
                    self.is_idle = false;
                    self.run();
                }
            });
            ui.add_enabled_ui(!self.is_idle, |ui| {
                if ui.button("Cancel").clicked() {
                    self.is_idle = true;
                }
            });
        });

        ui.separator();

        // let mut readonly_buffer = "abc";

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
