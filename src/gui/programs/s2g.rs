use eframe::egui::{self, ScrollArea};

use crate::gui::{
    constants::{PROGRAM_HEIGHT, PROGRAM_WIDTH},
    utils::preview_files_being_dropped,
    TabProgram,
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

// #[derive(Default)]
pub struct S2G {
    drag_and_drop: DragAndDrop,
    steps: Steps,
    output: String,
}

impl Default for S2G {
    fn default() -> Self {
        Self {
            drag_and_drop: Default::default(),
            steps: Default::default(),
            output: String::from(
                "\
std::array<float, 3> HwDLL::GetRenderedViewangles() {
	std::array<float, 3> res = {player.Viewangles[0], player.Viewangles[1], player.Viewangles[2]};

	if (!PitchOverrides.empty()) {
		res[0] = PitchOverrides[PitchOverrideIndex];
	}
	if (!RenderPitchOverrides.empty()) {
		res[0] = RenderPitchOverrides[RenderPitchOverrideIndex];
	}

	if (!TargetYawOverrides.empty()) {
		res[1] = TargetYawOverrides[TargetYawOverrideIndex];
	}
	if (!RenderYawOverrides.empty()) {
		res[1] = RenderYawOverrides[RenderYawOverrideIndex];
	}

	return res;
}
",
            ),
        }
    }
}

impl TabProgram for S2G {
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
        ui.label("Output: ");
        ui.separator();
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_sized(
                egui::vec2(PROGRAM_WIDTH, PROGRAM_HEIGHT / 3.),
                // Unironically the way to make textbox immutable, LMFAO
                egui::TextEdit::multiline(&mut self.output.as_str()),
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
