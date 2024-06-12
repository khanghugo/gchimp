use std::path::{Path, PathBuf};

use eframe::egui::{self, Context, RichText, ScrollArea, Ui};

use crate::{
    gui::{
        utils::{display_image_viewport_from_uri, preview_file_being_dropped},
        TabProgram,
    },
    modules::waddy::Waddy,
    utils::img_stuffs::any_format_to_png,
};

pub struct WaddyGui {
    instances: Vec<WaddyInstance>,
    extra_image_viewports: Vec<ExtraImageViewports>,
}

#[derive(Clone)]
struct ExtraImageViewports {
    uri: String,
    name: String,
}

struct WaddyInstance {
    path: PathBuf,
    waddy: Waddy,
    texture_tiles: Vec<TextureTile>,
}

struct TextureTile {
    index: usize,
    name: String,
    texture_bytes: &'static [u8],
    dimensions: (u32, u32),
    in_rename: bool,
    prev_name: String,
}

impl TextureTile {
    fn new(
        index: usize,
        name: impl AsRef<str> + Into<String>,
        texture_bytes: &'static [u8],
        dimensions: (u32, u32),
    ) -> Self {
        Self {
            index,
            name: name.into(),
            texture_bytes,
            dimensions,
            in_rename: false,
            prev_name: String::new(),
        }
    }
}

impl Default for WaddyGui {
    fn default() -> Self {
        Self {
            instances: vec![],
            extra_image_viewports: vec![],
        }
    }
}

static TEXTURE_PER_ROW: usize = 4;
static IMAGE_TILE_SIZE: f32 = 96.0;
static SUPPORTED_TEXTURE_FORMATS: &[&str] = &["png", "jpeg", "jpg", "bmp"];

impl WaddyGui {
    fn texture_tile(
        &mut self,
        ui: &mut Ui,
        instance_index: usize,
        texture_tile_index: usize,
    ) -> Option<usize> {
        let instance = &mut self.instances[instance_index];
        let texture_tile = &mut instance.texture_tiles[texture_tile_index];

        let mut texture_tile_to_delete: Option<usize> = None;

        // FIXME: reduce ram usage by at least 4 times
        egui::Grid::new(format!("tile{}", texture_tile.index))
            .num_columns(1)
            .max_col_width(IMAGE_TILE_SIZE)
            .spacing([0., 0.])
            .show(ui, |ui| {
                let uri_name = format!(
                    "{}-{}-{}-{}x{}",
                    instance_index,
                    texture_tile.index,
                    texture_tile.name,
                    texture_tile.dimensions.0,
                    texture_tile.dimensions.1
                );
                let uri = format!("bytes://{}", uri_name);
                let image = egui::Image::from_bytes(
                    uri.clone(),
                    egui::load::Bytes::Static(texture_tile.texture_bytes),
                );

                let clickable_image = ui.add_sized(
                    [IMAGE_TILE_SIZE, IMAGE_TILE_SIZE],
                    egui::ImageButton::new(image).frame(false).selected(false),
                );

                let mut context_menu_clicked = false;

                clickable_image.context_menu(|ui| {
                    ui.add_enabled(false, egui::Label::new(&texture_tile.name));

                    if ui.button("Rename").clicked() {
                        texture_tile.in_rename = true;
                        context_menu_clicked = true;
                        texture_tile.prev_name.clone_from(&texture_tile.name);
                        ui.close_menu();
                    }

                    if ui.button("Delete").clicked() {
                        texture_tile_to_delete = Some(texture_tile_index);
                        ui.close_menu();
                    }

                    if ui.button("Export").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&texture_tile.name)
                            .add_filter("All Files", &["bmp"])
                            .save_file()
                        {
                            // tODO TOAST
                            if let Err(err) = instance
                                .waddy
                                .dump_texture_to_file(texture_tile_index, path)
                            {
                                println!("{}", err);
                            }
                        }

                        ui.close_menu();
                    }
                });

                if clickable_image.double_clicked() {
                    self.extra_image_viewports.push(ExtraImageViewports {
                        uri,
                        name: uri_name,
                    });
                };

                ui.end_row();
                if texture_tile.in_rename {
                    let widget = ui.add(
                        egui::TextEdit::singleline(&mut texture_tile.name)
                            .font(egui::TextStyle::Small),
                    );

                    widget.request_focus();

                    if ui.input(|i| i.key_pressed(egui::Key::Escape))
                        || (widget.clicked_elsewhere() && !context_menu_clicked) // does not work because rename is clicked on the same tick
                        || widget.lost_focus()
                        || !widget.has_focus()
                    {
                        texture_tile.in_rename = false;
                        texture_tile.name.clone_from(&texture_tile.prev_name);
                    } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        // this is the only case where the name is changed successfully
                        texture_tile.in_rename = false;

                        if let Err(err) = instance
                            .waddy
                            .rename_texture(texture_tile_index, texture_tile.name.clone())
                        {
                            // TODO learn how to do toast
                            println!("{:?}", err);

                            texture_tile.name.clone_from(&texture_tile.prev_name);
                        } else if texture_tile.name.len() >= 16 {
                            println!("Texture name is too long");

                            texture_tile.name.clone_from(&texture_tile.prev_name);
                        }
                    }
                } else {
                    // beside the context menu, double click on the name would also enter rename mode
                    if ui
                        .label(custom_font(texture_tile.name.clone()))
                        .double_clicked()
                    {
                        texture_tile.in_rename = true;
                        texture_tile.prev_name.clone_from(&texture_tile.name);
                    };
                }

                ui.end_row();
                ui.label(custom_font(format!(
                    "{}x{}",
                    texture_tile.dimensions.0, texture_tile.dimensions.1
                )));
            });

        texture_tile_to_delete
    }

    fn display_image_viewports(&mut self, ctx: &Context) {
        self.extra_image_viewports = self
            .extra_image_viewports
            .iter()
            .filter(|uri| !display_image_viewport_from_uri(ctx, &uri.uri, &uri.name))
            .cloned()
            .collect::<Vec<ExtraImageViewports>>()
    }

    fn texture_grid(&mut self, ui: &mut Ui, instance_index: usize) {
        ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("waddy_grid")
                .num_columns(TEXTURE_PER_ROW)
                .spacing([2., 2.])
                .show(ui, |ui| {
                    let count = self.instances[instance_index].texture_tiles.len();

                    for texture_tile_index in 0..count {
                        if texture_tile_index % TEXTURE_PER_ROW == 0 && texture_tile_index != 0 {
                            ui.end_row()
                        }

                        if let Some(delete) =
                            self.texture_tile(ui, instance_index, texture_tile_index)
                        {
                            self.instances[instance_index].texture_tiles.remove(delete);
                            self.instances[instance_index]
                                .waddy
                                .remove_texture(texture_tile_index);
                            break;
                        }
                    }
                });
        });
    }

    // gui when there's WAD loaded
    fn editor_gui(&mut self, ui: &mut Ui, index: usize) {
        let mut should_close = false;

        ui.separator();

        ui.horizontal(|ui| {
            if self.editor_menu(ui, index) {
                should_close = true;
                return;
            }

            ui.label(self.instances[index].path.display().to_string())
                .on_hover_text(format!(
                    "{} textures",
                    self.instances[index].texture_tiles.len()
                ));
        });

        if should_close {
            return;
        }

        ui.separator();
        self.texture_grid(ui, index);

        let ctx = ui.ctx();

        self.display_image_viewports(ctx);

        preview_file_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            for item in &i.raw.dropped_files {
                if let Some(path) = &item.path {
                    if path.is_dir() {
                        return;
                    }

                    if let Some(ext) = path.extension() {
                        if ext == "wad" {
                            if let Err(err) = self.start_waddy_instance(path) {
                                // TODO TOAST
                                println!("{}", err);
                            }
                        } else if SUPPORTED_TEXTURE_FORMATS.contains(&ext.to_str().unwrap()) {
                            if let Err(err) = self.instances[index].waddy.add_texture(path) {
                                println!("{}", err);
                            } else if let Ok(bytes) = any_format_to_png(path) {
                                self.instances[index].texture_tiles.push(TextureTile::new(
                                    index,
                                    path.file_stem().unwrap().to_str().unwrap(),
                                    Box::leak(Box::new(bytes)),
                                    (512, 512),
                                ))
                            }
                        }
                    }
                }
            }
        });
    }

    // FIXME: it is ram guzzler
    fn start_waddy_instance(&mut self, path: &Path) -> eyre::Result<()> {
        let waddy = Waddy::from_file(path)?;
        let textures = waddy.dump_textures_to_png_bytes()?;

        let texture_tiles = textures
            .into_iter()
            .map(|(index, texture)| {
                // let leaked_bytes = texture.leak();

                let texture_bytes = Box::leak(Box::new(texture));

                TextureTile::new(
                    index,
                    waddy.wad().entries[index].texture_name(),
                    texture_bytes,
                    waddy.wad().entries[index].file_entry.dimensions(),
                )
            })
            .collect::<Vec<TextureTile>>();

        // TODO for the time being this can only open 1 wad file
        if !self.instances.is_empty() {
            self.instances.remove(0);
        }

        self.instances.push(WaddyInstance {
            path: path.to_path_buf(),
            waddy,
            texture_tiles,
        });

        Ok(())
    }

    fn menu_open(&mut self) -> bool {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            if path.extension().unwrap().to_str().unwrap() == "wad" {
                // todo toast
                if let Err(err) = self.start_waddy_instance(path.as_path()) {
                    println!("{}", err);
                } else {
                    return true;
                }
            }
        }

        false
    }

    fn editor_menu(&mut self, ui: &mut Ui, instance_index: usize) -> bool {
        let mut should_close = false;

        ui.menu_button("Menu", |ui| {
            if ui.button("New").clicked() {
                todo!()
            }

            if ui.button("Open").clicked() {
                should_close = self.menu_open();

                ui.close_menu();
            }

            // short circuit here so we won't render with the wrong `instance` next line.
            if should_close {
                return;
            }

            let instance = &mut self.instances[instance_index];

            if ui.button("Save").clicked() {
                // TODO TOAST TOAST
                if let Err(err) = instance.waddy.wad().write_to_file(instance.path.as_path()) {
                    println!("{}", err);
                }

                ui.close_menu();
            }

            if ui.button("Save As").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("All Files", &["wad"])
                    .set_file_name(instance.path.file_stem().unwrap().to_str().unwrap())
                    .save_file()
                {
                    // TODO TOAST TOAST
                    if let Err(err) = instance
                        .waddy
                        .wad()
                        .write_to_file(path.with_extension("wad"))
                    {
                        println!("{}", err);
                    } else {
                        // Change path to the current WAD file if we use Save As
                        instance.path = path;
                    }
                }

                ui.close_menu();
            }

            if ui.button("Close").clicked() {
                instance.texture_tiles.iter_mut().for_each(|tile| {
                    Box::into_raw(Box::new(tile.texture_bytes));
                });
                self.instances.remove(instance_index);

                should_close = true;

                ui.close_menu();
            }
        });

        should_close
    }
}

impl TabProgram for WaddyGui {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "Waddy".into()
    }

    fn tab_ui(&mut self, ui: &mut eframe::egui::Ui) -> egui_tiles::UiResponse {
        if !self.instances.is_empty() {
            self.editor_gui(ui, 0);
        } else {
            ui.separator();
            // UI when there is nothing
            if ui.button("Open").clicked() {
                self.menu_open();
            }

            ui.label("You can drag and drop too.");

            let ctx = ui.ctx();

            preview_file_being_dropped(ctx);

            // Collect dropped files:
            ctx.input(|i| {
                for item in &i.raw.dropped_files {
                    if let Some(path) = &item.path {
                        if path.is_dir() {
                            return;
                        }

                        if let Some(ext) = path.extension() {
                            if ext == "wad" {
                                if let Err(err) = self.start_waddy_instance(path) {
                                    // TODO TOAST
                                    println!("{}", err);
                                }
                            }
                        }
                    }
                }
            });
        }

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}

fn custom_font(s: impl Into<String>) -> RichText {
    egui::RichText::new(s).size(11.).small_raised().strong()
}
