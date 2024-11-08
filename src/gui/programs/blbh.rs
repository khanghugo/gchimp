use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use eframe::egui;

use crate::{
    config::Config,
    gui::{utils::preview_file_being_dropped, TabProgram},
    modules::blender_lightmap_baker_helper::{
        blender_lightmap_baker_helper, BLBHOptions, BLBH, BLBH_DEFAULT_UV_SHRINK_FACTOR,
    },
};

#[derive(Debug)]
pub struct BLBHGui {
    config: Config,
    smd_path: String,
    texture_path: String,
    options: BLBHOptions,
    shrink_value: String,
    check_shrink_value: bool,
    // origin: String,
    status: Arc<Mutex<String>>,
}

impl BLBHGui {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            smd_path: Default::default(),
            texture_path: Default::default(),
            options: BLBHOptions::default(),
            shrink_value: BLBH_DEFAULT_UV_SHRINK_FACTOR.to_string(),
            check_shrink_value: false,
            // origin: "0 0 0".to_string(),
            status: Arc::new(Mutex::new("Idle".to_string())),
        }
    }

    pub fn run(&mut self) {
        let smd_path = self.smd_path.clone();
        let texture_path = self.texture_path.clone();
        let options = self.options.clone();

        let Config {
            studiomdl,
            #[cfg(target_os = "linux")]
            wineprefix,
            ..
        } = self.config.clone();

        let status = self.status.clone();

        // options.origin = parse_triplet(&self.origin)
        //     .map(|res| res.into())
        //     .unwrap_or(DVec3::ZERO);

        "Running".clone_into(&mut status.lock().unwrap());

        let _join_handle = thread::spawn(move || {
            let mut blbh = BLBH {
                smd_path: PathBuf::from(smd_path),
                texture_path: PathBuf::from(texture_path),
                options,
            };

            blbh.options.studiomdl = studiomdl;

            #[cfg(target_os = "linux")]
            {
                blbh.options.wineprefix = wineprefix.unwrap();
            }

            match blender_lightmap_baker_helper(&blbh) {
                Ok(_) => {
                    let mut status = status.lock().unwrap();

                    "Done".clone_into(&mut status);
                }
                Err(err) => {
                    let mut status = status.lock().unwrap();

                    err.to_string().clone_into(&mut status);
                }
            }
        });
    }
}

impl TabProgram for BLBHGui {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "BLBH".into()
    }

    fn tab_ui(&mut self, ui: &mut eframe::egui::Ui) -> egui_tiles::UiResponse {
        ui.separator();
        ui.hyperlink_to(
            "Youtube link on how to make use of this",
            "https://www.youtube.com/watch?v=OFKPLioaS3I",
        );

        ui.separator();
        egui::Grid::new("Input smd and texture grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("SMD:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.smd_path).hint_text("Choose .smd file"),
                );
                if ui.button("Add").clicked() {
                    #[cfg(target_arch = "x86_64")]
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("SMD", &["smd"])
                        .pick_file()
                    {
                        if path.extension().is_some_and(|ext| ext == "smd") {
                            self.smd_path = path.display().to_string();
                        }
                    }
                }
                ui.end_row();

                ui.label("Texture:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.texture_path)
                        .hint_text("Choose an image file"),
                );
                if ui.button("Add").clicked() {
                    #[cfg(target_arch = "x86_64")]
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Image", &["png", "bmp,", "jpg", "jpeg"])
                        .pick_file()
                    {
                        self.texture_path = path.display().to_string();
                    }
                }
                ui.end_row();
            });

        ui.separator();
        ui.label("Options:");

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.options.convert_texture, "Convert texture")
                .on_hover_text("Splits 4096x4096 texture into 64 smaller compliant files");
            ui.checkbox(&mut self.options.convert_smd, "Convert SMD")
                .on_hover_text(
                    "Creates new SMD file that will use those new texture files accordingly",
                );
            ui.checkbox(&mut self.options.compile_model, "Compile MDL")
                .on_hover_text(
                    "Creates QC file and compiles the model with included studiomdl.exe",
                );
            ui.checkbox(&mut self.options.flat_shade, "Flat shade")
                .on_hover_text("Flags every texture with flat shade");
        });

        ui.horizontal(|ui| {
            ui.label("UV Shrink");
            // only check value if lost focus
            let text_editor = egui::TextEdit::singleline(&mut self.shrink_value).desired_width(80.);

            let text_editor_ui = ui.add(text_editor).on_hover_text(
                "\
UV coordinate from centroid will scale by this value. \n\
If your texture has weird seams, consider lowering this number. \n\
For best results, change this value by 1/512.",
            );

            if text_editor_ui.has_focus() {
                self.check_shrink_value = true;
            }

            if text_editor_ui.lost_focus() && self.check_shrink_value {
                let shrink_value = self
                    .shrink_value
                    .parse::<f32>()
                    .unwrap_or(BLBH_DEFAULT_UV_SHRINK_FACTOR);
                self.shrink_value = shrink_value.to_string();
                self.options.uv_shrink_factor = shrink_value;
                self.check_shrink_value = false;
            }

            if ui.button("Default").clicked() {
                self.shrink_value = BLBH_DEFAULT_UV_SHRINK_FACTOR.to_string()
            }

            // origin
            // ui.label("Origin");

            // let text_editor = egui::TextEdit::singleline(&mut self.origin).desired_width(96.)   ;
            // ui.add(text_editor)
            //     .on_hover_text("Origin of the model. Space separated.");
        });

        ui.separator();

        if ui.button("Run").clicked() {
            self.run();
        }

        let binding = self.status.lock().unwrap();
        let mut status_text = binding.as_str();
        ui.text_edit_multiline(&mut status_text);

        let ctx = ui.ctx();
        preview_file_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            for item in i.raw.dropped_files.clone() {
                if let Some(item) = item.path {
                    if item.is_file() {
                        if item.extension().is_some_and(|ext| ext == "smd") {
                            self.smd_path = item.to_str().unwrap().to_string();
                        } else {
                            self.texture_path = item.to_str().unwrap().to_string();
                        }
                    }
                }
            }
        });

        // Force continuous mode
        ctx.request_repaint();

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
