use std::path::{Path, PathBuf};

use eframe::egui::{self, Context, Modifiers, RichText, ScrollArea, Sense, Ui};
use wad::FileEntry;

use crate::{
    gui::{
        utils::{display_image_viewport_from_texture, preview_file_being_dropped, WadImage},
        TabProgram,
    },
    modules::waddy::Waddy,
};

pub struct WaddyGui {
    instances: Vec<WaddyInstance>,
    extra_image_viewports: Vec<WadImage>,
}

struct WaddyInstance {
    path: Option<PathBuf>,
    waddy: Waddy,
    texture_tiles: Vec<TextureTile>,
    // so the user can save the file
    is_changed: bool,
}

struct LoadedImage {
    image: WadImage,
}

struct TextureTile {
    index: usize,
    name: String,
    image: LoadedImage,
    dimensions: (u32, u32),
    in_rename: bool,
    prev_name: String,
}

impl TextureTile {
    fn new(
        index: usize,
        name: impl AsRef<str> + Into<String>,
        image: LoadedImage,
        dimensions: (u32, u32),
    ) -> Self {
        Self {
            index,
            name: name.into(),
            image,
            dimensions,
            in_rename: false,
            prev_name: String::new(),
        }
    }
}

#[allow(clippy::derivable_impls)]
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
    /// Returns index of texture to delete
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
        egui::Grid::new(format!("tile{}{}", texture_tile.index, texture_tile.name))
            .num_columns(1)
            .max_col_width(IMAGE_TILE_SIZE)
            .spacing([0., 0.])
            .show(ui, |ui| {
                let texture = texture_tile.image.image.texture();
                let dimensions = texture.size_vec2() / 512. * IMAGE_TILE_SIZE;

                let clickable_image = ui.add_sized(
                    [IMAGE_TILE_SIZE, IMAGE_TILE_SIZE],
                    egui::ImageButton::new(egui::Image::new((texture.id(), dimensions)))
                        .frame(false)
                        .selected(false),
                );

                let mut context_menu_clicked = false;

                clickable_image.context_menu(|ui| {
                    // if clicked then copy the name of the texture
                    if ui
                        .add(
                            egui::Label::new(&texture_tile.name)
                                .selectable(false)
                                .sense(Sense::click()),
                        )
                        .on_hover_text("Click to copy name")
                        .clicked()
                    {
                        ui.output_mut(|o| o.copied_text = texture_tile.name.to_string());
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("View").clicked() {
                        self.extra_image_viewports
                            .push(WadImage::new(texture_tile.image.image.texture()));
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Rename").clicked() {
                        texture_tile.in_rename = true;
                        context_menu_clicked = true;
                        texture_tile.prev_name.clone_from(&texture_tile.name);
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

                    ui.separator();

                    if ui.button("Delete").clicked() {
                        texture_tile_to_delete = Some(texture_tile_index);
                        ui.close_menu();
                    }
                });

                // double click wound bring a new viewport
                if clickable_image.double_clicked() {
                    self.extra_image_viewports
                        .push(WadImage::new(texture_tile.image.image.texture()));
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
                        } else {
                            // this means things are good
                            instance.is_changed = true;
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
            .filter(|wad_img| !display_image_viewport_from_texture(ctx, wad_img.texture()))
            .cloned()
            .collect::<Vec<WadImage>>()
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
                            self.instances[instance_index].is_changed = true;
                            break;
                        }
                    }
                });
        });
    }

    // gui when there's WAD loaded
    fn editor_gui(&mut self, ui: &mut Ui, instance_index: usize) {
        // should close to short-circuit the GUI and avoid accessing non existing info
        let mut should_close = false;

        ui.separator();

        ui.horizontal(|ui| {
            if self.editor_menu(ui, instance_index) {
                should_close = true;
                return;
            }

            if let Some(path) = &self.instances[instance_index].path {
                let wad_file_name = path.display().to_string();

                let wad_file_name = if self.instances[instance_index].is_changed {
                    format!("*{wad_file_name}")
                } else {
                    wad_file_name
                };

                ui.add(
                    egui::Label::new(wad_file_name)
                        .sense(Sense::hover())
                        .truncate(),
                )
                .on_hover_text(format!(
                    "{} textures",
                    self.instances[instance_index].texture_tiles.len()
                ));
            } else {
                ui.label("New WAD");
                ui.label(format!(
                    "{} textures",
                    self.instances[instance_index].texture_tiles.len()
                ));
            }
        });

        if should_close {
            return;
        }

        ui.separator();
        self.texture_grid(ui, instance_index);

        let ctx = ui.ctx();

        self.display_image_viewports(ctx);

        // CTRL+S
        ui.input(|i| {
            if i.modifiers.matches_exact(Modifiers::CTRL) && i.key_released(egui::Key::S) {
                self.menu_save(instance_index);
            }
        });

        preview_file_being_dropped(ctx);

        // Collect dropped files:
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());

        for item in &dropped_files {
            if let Some(path) = &item.path {
                if path.is_dir() {
                    continue;
                }

                if let Some(ext) = path.extension() {
                    // if new wad file is dropped, we open that wad file instead
                    if ext == "wad" {
                        if let Err(err) = self.start_waddy_instance(ui, Some(path)) {
                            // TODO TOAST
                            println!("{}", err);
                        }
                    // if an image file is dropped, we will add that to the current wad file
                    } else if SUPPORTED_TEXTURE_FORMATS.contains(&ext.to_str().unwrap()) {
                        if let Err(err) = self.instances[instance_index].waddy.add_texture(path) {
                            println!("{}", err);
                        } else {
                            // after adding a new texture, we have to update the gui to include that new file
                            let new_entry = self.instances[instance_index]
                                .waddy
                                .wad()
                                .entries
                                .last()
                                .unwrap();

                            let texture_name = new_entry.directory_entry.texture_name.get_string();
                            let dimensions =
                                if let FileEntry::MipTex(miptex) = &new_entry.file_entry {
                                    (miptex.width, miptex.height)
                                } else {
                                    unreachable!()
                                };
                            let wad_image = if let FileEntry::MipTex(miptex) = &new_entry.file_entry
                            {
                                WadImage::from_wad_image(
                                    ui,
                                    texture_name.clone(),
                                    miptex.mip_images[0].data.get_bytes(),
                                    miptex.palette.get_bytes(),
                                    dimensions,
                                )
                            } else {
                                unreachable!()
                            };

                            self.instances[instance_index]
                                .texture_tiles
                                .push(TextureTile::new(
                                    instance_index,
                                    texture_name,
                                    LoadedImage { image: wad_image },
                                    dimensions,
                                ));

                            self.instances[instance_index].is_changed = true;
                        }
                    }
                }
            }
        }
    }

    // FIXME: it is ram guzzler
    fn start_waddy_instance(&mut self, ui: &mut Ui, path: Option<&Path>) -> eyre::Result<()> {
        let waddy = if let Some(path) = path {
            Waddy::from_file(path)?
        } else {
            Waddy::new()
        };

        let texture_tiles = waddy
            .wad()
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if let FileEntry::MipTex(miptex) = &entry.file_entry {
                    let loaded_image = WadImage::from_wad_image(
                        ui,
                        entry.directory_entry.texture_name.get_string(),
                        miptex.mip_images[0].data.get_bytes(),
                        miptex.palette.get_bytes(),
                        (miptex.width, miptex.height),
                    );

                    return Some(TextureTile::new(
                        index,
                        waddy.wad().entries[index].texture_name(),
                        LoadedImage {
                            image: loaded_image,
                        },
                        waddy.wad().entries[index].file_entry.dimensions(),
                    ));
                    // None
                }

                None
            })
            .collect::<Vec<TextureTile>>();

        // TODO for the time being this can only open 1 wad file
        if !self.instances.is_empty() {
            self.instances.remove(0);
        }

        self.instances.push(WaddyInstance {
            path: path.map(|path| path.to_owned()),
            waddy,
            texture_tiles,
            is_changed: false,
        });

        Ok(())
    }

    fn menu_open(&mut self, ui: &mut Ui) -> bool {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            if path.extension().unwrap().to_str().unwrap() == "wad" {
                // todo toast
                if let Err(err) = self.start_waddy_instance(ui, Some(path.as_path())) {
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
                let _ = self.start_waddy_instance(ui, None);

                ui.close_menu();
            }

            if ui.button("Open").clicked() {
                should_close = self.menu_open(ui);

                ui.close_menu();
            }

            ui.separator();

            if ui.button("Save (Ctrl+S)").clicked() {
                self.menu_save(instance_index);

                ui.close_menu();
            }

            if ui.button("Save As").clicked() {
                self.menu_save_as_dialogue(instance_index);

                ui.close_menu();
            }

            if ui.button("Export").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    // TODO TOAST TOAST
                    if let Err(err) = self.instances[instance_index]
                        .waddy
                        .dump_textures_to_files(path)
                    {
                        println!("{}", err);
                    }
                }

                ui.close_menu();
            }

            ui.separator();

            if ui.button("Close").clicked() {
                self.instances[instance_index]
                    .texture_tiles
                    .iter_mut()
                    .for_each(|tile| {
                        // tile.
                    });
                self.instances.remove(instance_index);

                should_close = true;

                ui.close_menu();
            }
        });

        should_close
    }

    fn menu_save(&mut self, instance_index: usize) {
        if let Some(path) = &self.instances[instance_index].path {
            // TODO TOAST TOAST
            if let Err(err) = self.instances[instance_index]
                .waddy
                .wad()
                .write_to_file(path.as_path())
            {
                println!("{}", err);
            } else {
                self.instances[instance_index].is_changed = false;
            }
        } else {
            self.menu_save_as_dialogue(instance_index);
        }
    }

    fn menu_save_as_dialogue(&mut self, instance_index: usize) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("All Files", &["wad"])
            .set_file_name(if let Some(path) = &self.instances[instance_index].path {
                path.file_stem().unwrap().to_str().unwrap()
            } else {
                ""
            })
            .save_file()
        {
            // TODO TOAST TOAST
            if let Err(err) = self.instances[instance_index]
                .waddy
                .wad()
                .write_to_file(path.with_extension("wad"))
            {
                println!("{}", err);
            } else {
                // Change path to the current WAD file if we use Save As
                self.instances[instance_index].path = Some(path);
                self.instances[instance_index].is_changed = false;
            }
        }
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
            ui.menu_button("Menu", |ui| {
                if ui.button("New").clicked() {
                    let _ = self.start_waddy_instance(ui, None);

                    ui.close_menu();
                }

                if ui.button("Open").clicked() {
                    self.menu_open(ui);

                    ui.close_menu();
                }
            });

            ui.separator();
            ui.label("You can drag and drop too.");

            let ctx = ui.ctx();

            preview_file_being_dropped(ctx);

            // Collect dropped files:
            let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());

            for item in &dropped_files {
                if let Some(path) = &item.path {
                    if path.is_dir() {
                        continue;
                    }

                    if let Some(ext) = path.extension() {
                        if ext == "wad" {
                            if let Err(err) = self.start_waddy_instance(ui, Some(path)) {
                                // TODO TOAST
                                println!("{}", err);
                            }
                        }
                    }
                }
            }
        }

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}

fn custom_font(s: impl Into<String>) -> RichText {
    egui::RichText::new(s).size(11.).small_raised().strong()
}
