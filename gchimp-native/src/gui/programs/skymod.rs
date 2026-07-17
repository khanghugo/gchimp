use std::{
    array::from_fn,
    mem,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use eframe::egui::{self, ScrollArea, Vec2};

use common::{constants::MAX_GOLDSRC_TEXTURE_SIZE, img_stuffs::hdri_to_cubemap};

use gchimp::modules::skymod::{SkyModOptions, map_file_name_to_index, skymod};
use image::{Rgba32FImage, RgbaImage};

use crate::{
    config::Config,
    gui::{
        TabProgram,
        constants::{HDR_FORMATS, IMAGE_FORMATS, PROGRAM_HEIGHT, PROGRAM_WIDTH},
        utils::{load_rgba8_to_egui_texture, preview_file_being_dropped},
    },
};

static FACE_SIZE: f32 = 94.;

struct HDRI {
    path: PathBuf,
    image: Rgba32FImage,
    // don't store processed cube map because it saves RAM :DDDD
    exposure: f32,
}

#[derive(Default)]
struct Cubemap {
    paths: [Option<PathBuf>; 6],
    cubemap: [Option<RgbaImage>; 6],
}

pub struct SkyModGui {
    #[allow(dead_code)]
    app_config: Config,
    displayed_textures: DisplayedTexture,
    texture_handles: [Option<egui::TextureHandle>; 6],
    options: SkyModOptions,
    // need this because text input is string and we want number
    skybox_size: String,
    texture_per_face: String,
    idle: bool,
    status: Arc<Mutex<String>>,
}

#[derive(Default)]
enum DisplayedTexture {
    #[default]
    None,
    Cubemap(Cubemap),
    HDRI(HDRI),
}

impl SkyModGui {
    pub fn new(app_config: Config) -> Self {
        Self {
            app_config,
            displayed_textures: DisplayedTexture::None,
            texture_handles: [const { None }; 6],
            options: SkyModOptions::default(),
            skybox_size: String::from("131072"),
            texture_per_face: String::from("1"),
            idle: true,
            status: Arc::new(Mutex::new(String::from("Idle"))),
        }
    }

    fn run(&mut self) {
        let texture_per_face = self.texture_per_face.parse::<u32>();
        let skybox_size = self.skybox_size.parse::<u32>();

        let output = self.status.clone();
        "Running".clone_into(&mut output.lock().unwrap());

        if texture_per_face.is_err() {
            "Texture per face is not a number".clone_into(&mut output.lock().unwrap());
            return;
        }

        if skybox_size.is_err() {
            "Skybox size is not a number".clone_into(&mut output.lock().unwrap());
            return;
        }

        if self.options.output_name.is_empty() {
            "Output name is empty".clone_into(&mut output.lock().unwrap());
            return;
        }

        self.idle = false;

        let options = self.options.clone();

        // let displayed_textures = std::mem::take(&mut self.displayed_textures);
        let (save_path, cubemap) = match &self.displayed_textures {
            DisplayedTexture::None => {
                "No textures selected".clone_into(&mut output.lock().unwrap());
                self.idle = true;
                return;
            }
            DisplayedTexture::Cubemap(cubemap) => {
                if cubemap.cubemap.iter().any(|x| x.is_none()) {
                    "Not all 6 textures are selected".clone_into(&mut output.lock().unwrap());
                    self.idle = true;
                    return;
                }

                (
                    cubemap.paths[0].clone().unwrap(),
                    from_fn(|i| cubemap.cubemap[i].clone().unwrap()),
                )
            }
            DisplayedTexture::HDRI(hdri) => (
                hdri.path.to_owned(),
                hdri_to_cubemap(
                    &hdri.image,
                    options.texture_per_side * MAX_GOLDSRC_TEXTURE_SIZE,
                    hdri.exposure,
                )
                .map(|(_, x)| x),
            ),
        };

        let res = skymod(cubemap, options.clone());

        match res {
            Ok(mdls) => {
                mdls.into_iter().enumerate().for_each(|(idx, x)| {
                    let res = x.write_to_file(
                        save_path
                            .with_file_name(format!("{}{}", options.output_name, idx))
                            .with_extension("mdl"),
                    );

                    match res {
                        Ok(_) => {
                            "OK".clone_into(&mut output.lock().unwrap());
                        }
                        Err(err) => {
                            format!("{}", err).clone_into(&mut output.lock().unwrap());
                        }
                    }
                });
            }
            Err(err) => {
                format!("{}", err).clone_into(&mut output.lock().unwrap());
            }
        }

        self.idle = true;
    }

    fn load_image_to_tile(
        &mut self,
        ui: &mut eframe::egui::Ui,
        path: &Path,
        index: usize,
    ) -> Option<()> {
        let img = match image::open(path) {
            Ok(img) => img,
            Err(err) => {
                let output = self.status.clone();
                format!("{}", err).clone_into(&mut output.lock().unwrap());
                return None;
            }
        };

        if HDR_FORMATS.contains(
            &path
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("dummy dummm"),
        ) {
            let rgba32f = img.to_rgba32f();

            let res = self.displayed_textures = DisplayedTexture::HDRI(HDRI {
                path: path.to_path_buf(),
                image: rgba32f,
                exposure: 1., // default exposure
            });

            // update display
            self.update_hdri_display(ui);

            res
        } else {
            let img = img.to_rgba8();
            let pathbuf = path.to_path_buf();

            // load to gpu to display
            // so there are 2 copies
            let handle = load_rgba8_to_egui_texture(ui, &path.display().to_string(), &img).unwrap();
            self.texture_handles[index] = handle.into();

            self.displayed_textures = match mem::take(&mut self.displayed_textures) {
                DisplayedTexture::None | DisplayedTexture::HDRI(_) => DisplayedTexture::Cubemap({
                    let mut res = Cubemap::default();

                    res.cubemap[index] = img.into();
                    res.paths[index] = pathbuf.into();

                    res
                }),
                DisplayedTexture::Cubemap(mut cubemap) => {
                    cubemap.cubemap[index] = img.into();
                    cubemap.paths[index] = pathbuf.into();

                    DisplayedTexture::Cubemap(cubemap)
                }
            };
        }

        Some(())
    }

    fn selectable_face(&mut self, ui: &mut eframe::egui::Ui, index: usize, text: &str) {
        let button = if let Some(handle) = &self.texture_handles[index] {
            // now path is valid, check if the texture is loaded
            // otherwise we load new texture
            let image = egui::Image::new((
                handle.id(),
                Vec2 {
                    x: FACE_SIZE,
                    y: FACE_SIZE,
                },
            ));

            ui.add_sized(
                [FACE_SIZE, FACE_SIZE],
                egui::Button::image(image).frame(false),
            )
        } else {
            ui.add_sized([FACE_SIZE, FACE_SIZE], egui::Button::new(text))
        };

        // hand select like this is less likely b ut i still have to support the behavior.....
        // i start to see the technical debt here
        // who needs Ai to accummulate debt?
        if button.clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Image and HDR", &[IMAGE_FORMATS, HDR_FORMATS].concat())
                .pick_file()
        {
            self.load_image_to_tile(ui, &path, index);
        };
    }

    // hdr slider is not very responsive because we don't render 3d :(
    fn update_hdri_display(&mut self, ui: &mut eframe::egui::Ui) {
        if let DisplayedTexture::HDRI(hdri) = &self.displayed_textures {
            // doesn't need high cube dimension here
            let cubemap = hdri_to_cubemap(&hdri.image, 256, hdri.exposure).map(|(_, x)| x);

            for (texture_index, texture) in cubemap.into_iter().enumerate() {
                let handle =
                    load_rgba8_to_egui_texture(ui, &hdri.path.display().to_string(), &texture)
                        .unwrap();
                self.texture_handles[texture_index] = handle.into();
            }
        }
    }
}

impl TabProgram for SkyModGui {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "SkyMod".into()
    }

    fn tab_ui(&mut self, ui: &mut eframe::egui::Ui) -> egui_tiles::UiResponse {
        ui.separator();
        ui.ctx().forget_all_images();

        egui::Grid::new("Texture grid")
            .num_columns(4)
            // .min_col_width(0.)
            // .min_row_height(0.)
            // .max_col_width(0.)
            .spacing([1., 1.])
            .show(ui, |ui| {
                ui.label("");
                if ui.button("Reset").clicked() {
                    self.displayed_textures = DisplayedTexture::None;

                    (0..self.texture_handles.len())
                        .for_each(|idx| self.texture_handles[idx] = None);
                };
                self.selectable_face(ui, 0, "U");
                ui.end_row();

                self.selectable_face(ui, 1, "L");
                self.selectable_face(ui, 4, "B");
                self.selectable_face(ui, 3, "R");
                self.selectable_face(ui, 2, "F");
                ui.end_row();

                ui.label("");
                ui.label("");
                self.selectable_face(ui, 5, "D");
                ui.end_row();
            });

        ui.separator();

        // exposure slider is here becuase i don't know where to have it
        // dumb, optional ui.horizontal here can be empty and it leads to empty space in ui
        // cannot draw it optionally that depends on self because of borrowing
        // i commit a sin
        let should_draw_hdri_ui = matches!(self.displayed_textures, DisplayedTexture::HDRI(_));

        if should_draw_hdri_ui {
            ui.horizontal(|ui| {
        if let DisplayedTexture::HDRI(hdri) = &mut self.displayed_textures {
            ui.label("Exposure");

            let hdri_exposure_value = hdri.exposure; // copy

            let slider = egui::Slider::new(&mut hdri.exposure, -15.0..=15.0).max_decimals(3);

            let res = ui.add(slider);

            if ui.button("Export HDR Cubemap").on_hover_text("Export HDRI to 6 images in HDRI folder. Can change resolution through \"Texture per face\"").clicked() {
                    let output_folder = hdri.path.parent().unwrap();
                    let file_name = self.options.output_name.clone();

                    // redo the calculation here because the texture lives in GPU now
                    let cubemap = hdri_to_cubemap(
                        &hdri.image,
                        512 * (self.options.texture_per_side as f32).sqrt().round() as u32,
                        hdri_exposure_value,
                    );

                    for (suffix, img) in cubemap {
                        let _ = img.save(output_folder.join(format!("{}_{}",&file_name, suffix)).with_extension("png"));
                    }
                }

            // only update once dragging stops
            if res.drag_stopped() {
                self.update_hdri_display(ui);
            }
        }
            });
        }

        ui.horizontal(|ui| {
            egui::Grid::new("option grid")
                .num_columns(4)
                .show(ui, |ui| {
                    ui.label("Texture per face:");
                    ui.text_edit_singleline(&mut self.texture_per_face)
                        .on_hover_text(
                            "\
How many textures should each skybox face have? \n
It should be a perfect square (such as 1, 4, 9, 16, ..) \n
If a model has more than 64 textures, it will be split into smaller models",
                        );
                    ui.label("Skybox size:");
                    ui.text_edit_singleline(&mut self.skybox_size)
                        .on_hover_text("The size of the model");
                });
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.options.flatshade, "Flat shade")
                .on_hover_text(
                    "\
Mark texture with flatshade flag. \n
Recommeded to leave it checked for uniformly lit texture.",
                );

            ui.label("Output name: ");
            ui.text_edit_singleline(&mut self.options.output_name)
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(self.idle, |ui| {
                    if ui.button("Run").clicked() {
                        // TODO make it truly multithreaded like s2g
                        // kindda lazy to do it tbh
                        // it works ok enough and there isn't much on the output to report
                        self.run();
                    }
                });
                ui.add_enabled_ui(!self.idle, |ui| {
                    if ui.button("Cancel").clicked() {
                        self.idle = true;
                    }
                });
            });
        });

        ui.separator();

        let binding = self.status.lock().unwrap();
        let mut readonly_buffer = binding.as_str();

        ScrollArea::vertical().show(ui, |ui| {
            ui.add_sized(
                egui::vec2(PROGRAM_WIDTH, PROGRAM_HEIGHT / 16.),
                egui::TextEdit::multiline(&mut readonly_buffer),
            );
        });

        drop(binding); // drop it because borrowing

        // drag and drop stuff
        let ctx = ui.ctx();
        preview_file_being_dropped(ctx);

        // Collect dropped files:
        let collected_paths = ctx.input(|i| {
            let items = i.raw.dropped_files.clone();
            let paths = items
                .iter()
                .filter_map(|e| e.path.clone())
                .collect::<Vec<PathBuf>>();

            paths
        });

        collected_paths.iter().for_each(|path| {
            let img_index = map_file_name_to_index(path);

            if self
                .load_image_to_tile(ui, path, img_index as usize)
                .is_none()
            {
                return;
            }

            let skybox_name = path.file_stem().unwrap().to_str().unwrap();
            let skybox_name = &skybox_name[..skybox_name.len() - 2];
            self.options.output_name = skybox_name.to_string();
        });

        // make it continuous
        ui.ctx().request_repaint();

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
