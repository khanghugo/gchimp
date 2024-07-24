use std::path::{Path, PathBuf};

use eframe::egui::{self, Context, Modifiers, RichText, ScrollArea, Sense, Ui};
use wad::FileEntry;

use rayon::prelude::*;

use crate::{
    gui::{
        constants::{PROGRAM_HEIGHT, PROGRAM_WIDTH},
        utils::{display_image_viewport_from_texture, preview_file_being_dropped, WadImage},
        TabProgram,
    },
    modules::waddy::Waddy,
};

pub struct WaddyGui {
    instances: Vec<WaddyInstance>,
    extra_image_viewports: Vec<WadImage>,
    /// 32x32 texture on 512x512 grid is VERY TINY
    fit_texture: bool,
}

struct WaddyInstance {
    path: Option<PathBuf>,
    waddy: Waddy,
    texture_tiles: Vec<TextureTile>,
    // so the user can save the file
    is_changed: bool,
    selected: Vec<usize>,
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
            fit_texture: true,
        }
    }
}

static BASE_IMAGE_TILE_SIZE: f32 = 96.0;
static SUPPORTED_TEXTURE_FORMATS: &[&str] = &["png", "jpeg", "jpg", "bmp"];

impl WaddyGui {
    /// Returns index of texture to delete
    fn texture_tile(
        &mut self,
        ui: &mut Ui,
        instance_index: usize,
        texture_tile_index: usize,
        image_tile_size: f32,
    ) -> Option<Vec<usize>> {
        let instance = &mut self.instances[instance_index];

        let mut texture_tile_to_delete: Option<Vec<usize>> = None;

        // FIXME: reduce ram usage by at least 4 times
        let current_id = egui::Id::new(format!(
            "{}{}",
            instance.texture_tiles[texture_tile_index].index,
            instance.texture_tiles[texture_tile_index].name
        ));

        let is_selected = instance.selected.contains(&texture_tile_index);
        let selected_color = ui.style().visuals.selection.bg_fill;

        egui::Grid::new(current_id)
            .num_columns(1)
            .max_col_width(image_tile_size)
            .spacing([0., 0.])
            .with_row_color(move |_row, _style| {
                if is_selected {
                    Some(selected_color)
                } else {
                    None
                }
            })
            .show(ui, |ui| {
                let texture = instance.texture_tiles[texture_tile_index]
                    .image
                    .image
                    .texture();
                let dimensions = if self.fit_texture {
                    let dimensions = texture.size_vec2();
                    let bigger = dimensions.x.max(dimensions.y);

                    dimensions / bigger
                } else {
                    texture.size_vec2() / 512.
                } * image_tile_size;

                let clickable_image = ui.add_sized(
                    [image_tile_size, image_tile_size],
                    egui::ImageButton::new(egui::Image::new((texture.id(), dimensions)))
                        .frame(false)
                        .selected(false),
                );

                let mut context_menu_clicked = false;

                clickable_image.context_menu(|ui| {
                    let current_tile = &mut instance.texture_tiles[texture_tile_index];

                    // if clicked then copy the name of the texture
                    if ui
                        .add(
                            egui::Label::new(&current_tile.name)
                                .selectable(false)
                                .sense(Sense::click()),
                        )
                        .on_hover_text("Click to copy name")
                        .clicked()
                    {
                        ui.output_mut(|o| o.copied_text = current_tile.name.to_string());
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("View").clicked() {
                        self.extra_image_viewports
                            .push(WadImage::new(current_tile.image.image.texture()));
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Rename").clicked() {
                        current_tile.in_rename = true;
                        context_menu_clicked = true;

                        current_tile.prev_name.clone_from(&current_tile.name);
                        ui.close_menu();
                    }

                    // export when there's lots of selected or not
                    if instance.selected.is_empty() {
                        if ui.button("Export").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name(&instance.texture_tiles[texture_tile_index].name)
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
                    } else {
                        // when lots of selected there will be option to deselect
                        if ui.button("Deselect all").clicked() {
                            instance.selected.clear();
                            ui.close_menu();
                        }

                        if ui
                            .button(format!("Export ({})", instance.selected.len()))
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                instance
                                    .selected
                                    .par_iter()
                                    .for_each(|&texture_tile_index| {
                                        let current_tile =
                                            &instance.texture_tiles[texture_tile_index];

                                        let texture_file_name = &current_tile.name;

                                        // tODO TOAST
                                        if let Err(err) = instance.waddy.dump_texture_to_file(
                                            texture_tile_index,
                                            path.join(texture_file_name),
                                        ) {
                                            println!("{}", err);
                                        };
                                    });
                            }

                            ui.close_menu();
                        }
                    }

                    ui.separator();

                    // delete when lots of selected or not
                    if instance.selected.is_empty() {
                        if ui.button("Delete").clicked() {
                            texture_tile_to_delete = Some(vec![texture_tile_index]);
                            ui.close_menu();
                        }
                    } else if ui
                        .button(format!("Delete ({})", instance.selected.len()))
                        .clicked()
                    {
                        texture_tile_to_delete = Some(instance.selected.clone());
                        instance.selected.clear();

                        ui.close_menu()
                    }
                });

                // if left clicked, add to the list of selected
                if clickable_image.clicked() {
                    if is_selected {
                        instance.selected.remove(
                            instance
                                .selected
                                .iter()
                                .position(|&idx| idx == texture_tile_index)
                                .unwrap(),
                        );
                    } else {
                        instance.selected.push(texture_tile_index);
                    }
                }

                // middle click wound bring a new viewport
                if clickable_image.middle_clicked() {
                    self.extra_image_viewports.push(WadImage::new(
                        instance.texture_tiles[texture_tile_index]
                            .image
                            .image
                            .texture(),
                    ));
                };

                ui.end_row();

                let current_tile = &mut instance.texture_tiles[texture_tile_index];

                if current_tile.in_rename {
                    let widget = ui.add(
                        egui::TextEdit::singleline(&mut current_tile.name)
                            .font(egui::TextStyle::Small),
                    );

                    widget.request_focus();

                    if ui.input(|i| i.key_pressed(egui::Key::Escape))
                                || (widget.clicked_elsewhere() && !context_menu_clicked) // does not work because rename is clicked on the same tick
                                || widget.lost_focus()
                                || !widget.has_focus()
                    {
                        current_tile.in_rename = false;
                        current_tile.name.clone_from(&current_tile.prev_name);
                    } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        // this is the only case where the name is changed successfully
                        current_tile.in_rename = false;

                        if let Err(err) = instance
                            .waddy
                            .rename_texture(texture_tile_index, current_tile.name.clone())
                        {
                            // TODO learn how to do toast
                            println!("{:?}", err);

                            current_tile.name.clone_from(&current_tile.prev_name);
                        } else if current_tile.name.len() >= 16 {
                            println!("Texture name is too long");

                            current_tile.name.clone_from(&current_tile.prev_name);
                        } else {
                            // this means things are good
                            instance.is_changed = true;
                        }
                    }
                } else {
                    // beside the context menu, double click on the name would also enter rename mode
                    if ui
                        .label(custom_font(current_tile.name.clone()))
                        .double_clicked()
                    {
                        current_tile.in_rename = true;
                        current_tile.prev_name.clone_from(&current_tile.name);
                    };
                }

                ui.end_row();
                ui.label(custom_font(format!(
                    "{}x{}",
                    current_tile.dimensions.0, current_tile.dimensions.1
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
        let image_tile_size =
            BASE_IMAGE_TILE_SIZE * ui.ctx().options(|options| options.zoom_factor);
        let texture_per_row = ((ui.min_size().x / image_tile_size).floor() as usize).max(4);

        ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("waddy_grid")
                .num_columns(texture_per_row)
                .spacing([2., 2.])
                .show(ui, |ui| {
                    let count = self.instances[instance_index].texture_tiles.len();

                    for texture_tile_index in 0..count {
                        if texture_tile_index % texture_per_row == 0 && texture_tile_index != 0 {
                            ui.end_row()
                        }

                        if let Some(mut to_delete) = self.texture_tile(
                            ui,
                            instance_index,
                            texture_tile_index,
                            image_tile_size,
                        ) {
                            to_delete.sort();

                            to_delete.iter().rev().for_each(|&delete| {
                                self.instances[instance_index].texture_tiles.remove(delete);
                                self.instances[instance_index].waddy.remove_texture(delete);
                            });

                            self.instances[instance_index].is_changed = true;
                            break;
                        }
                    }
                });
        });
    }

    // gui when there's WAD loaded
    fn instance_ui(&mut self, ui: &mut Ui, instance_index: usize) {
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

        self.instances.push(WaddyInstance {
            path: path.map(|path| path.to_owned()),
            waddy,
            texture_tiles,
            is_changed: false,
            selected: vec![],
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

            if ui.button("Export All").clicked() {
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

            ui.menu_button("Options", |ui| {
                if ui.checkbox(&mut self.fit_texture, "Fit texture").clicked() {
                    ui.close_menu();
                }
            });

            ui.separator();

            if ui.button("Close").clicked() {
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

    fn empy_instance_ui(&mut self, ui: &mut egui::Ui) {
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
        ui.label("Drag and drop a WAD file to start");

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
}

impl TabProgram for WaddyGui {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "Waddy".into()
    }

    fn tab_ui(&mut self, ui: &mut eframe::egui::Ui) -> egui_tiles::UiResponse {
        if !self.instances.is_empty() {
            self.instance_ui(ui, 0);

            let ctx = ui.ctx();

            // show other instances in different viewports
            let to_remove = (1..self.instances.len())
                .filter(|instance_index| {
                    // let instance_name = format!("waddygui_instance{}", instance_index);
                    let instance_name = if let Some(path) = &self.instances[*instance_index].path {
                        path.display().to_string()
                    } else {
                        format!("waddygui_instance{}", instance_index)
                    };

                    ctx.show_viewport_immediate(
                        egui::ViewportId::from_hash_of(&instance_name),
                        egui::ViewportBuilder::default()
                            .with_title(instance_name)
                            .with_inner_size(
                                [PROGRAM_WIDTH, PROGRAM_HEIGHT], // border :()
                            ),
                        |ctx, _class| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                self.instance_ui(ui, *instance_index);

                                if ctx.input(|i| {
                                    i.viewport().close_requested()
                                        || i.key_pressed(egui::Key::Escape)
                                }) {
                                    return true;
                                };

                                false
                            })
                        },
                    )
                    .inner
                })
                .collect::<Vec<usize>>();

            to_remove.into_iter().rev().for_each(|index| {
                self.instances.remove(index);
            });
        } else {
            self.empy_instance_ui(ui);
        }

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}

fn custom_font(s: impl Into<String>) -> RichText {
    egui::RichText::new(s).size(11.).small_raised().strong()
}
