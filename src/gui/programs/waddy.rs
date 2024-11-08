use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use eframe::egui::{self, Context, Modifiers, RichText, ScrollArea, Sense, Ui};
use image::{ImageBuffer, RgbaImage};
use wad::types::FileEntry;

use rayon::prelude::*;

use crate::{
    gui::{
        constants::{PROGRAM_HEIGHT, PROGRAM_WIDTH},
        utils::{display_image_viewport_from_texture, preview_file_being_dropped, WadImage},
        TabProgram,
    },
    modules::waddy::Waddy,
    persistent_storage::PersistentStorage,
};

pub struct WaddyGui {
    instances: Vec<WaddyInstance>,
    extra_image_viewports: Vec<WadImage>,
    /// 32x32 texture on 512x512 grid is VERY TINY
    fit_texture: bool,
    persistent_storage: Arc<Mutex<PersistentStorage>>,
}

struct WaddyInstance {
    path: Option<PathBuf>,
    waddy: Waddy,
    texture_tiles: Vec<TextureTile>,
    // so the user can save the file
    is_changed: bool,
    selected: Vec<usize>,
    to_delete: Vec<usize>,
    search: SearchBar,
}

struct SearchBar {
    enable: bool,
    text: String,
    // dirty trick to focus on spawn
    should_focus: bool,
    // regain focus when ctrl+f again
    has_focus: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for SearchBar {
    fn default() -> Self {
        Self {
            enable: false,
            text: String::new(),
            should_focus: false,
            has_focus: false,
        }
    }
}

struct TextureTile {
    index: usize,
    wad_image: WadImage,
    in_rename: bool,
    prev_name: String,
}

impl TextureTile {
    fn new(index: usize, wad_image: WadImage) -> Self {
        Self {
            index,
            wad_image,
            in_rename: false,
            prev_name: String::new(),
        }
    }

    fn name(&self) -> &String {
        self.wad_image.name()
    }

    fn name_mut(&mut self) -> &mut String {
        self.wad_image.name_mut()
    }

    fn dimensions(&self) -> (u32, u32) {
        self.wad_image.dimensions()
    }

    #[allow(dead_code)]
    fn texture(&self) -> &egui::TextureHandle {
        self.wad_image.texture()
    }
}

const BASE_IMAGE_TILE_SIZE: f32 = 96.0;
const SUPPORTED_TEXTURE_FORMATS: &[&str] = &["png", "jpeg", "jpg", "bmp", "vtf"];

const PERSISTENT_STORAGE_RECENTLY_USED_UPDATE_ERROR: &str =
    "cannot update recently used wad for Waddy";

impl WaddyGui {
    pub fn new(persistent_storage: Arc<Mutex<PersistentStorage>>) -> Self {
        Self {
            instances: vec![],
            extra_image_viewports: vec![],
            fit_texture: true,
            persistent_storage,
        }
    }

    // returns true if any context menu button is clicked
    fn texture_tile_context_menu(&mut self, ui: &mut Ui, instance_index: usize) -> bool {
        let mut context_menu_clicked = false;

        // if a tile is selected, then we will do everything over that tile
        // a tile is always guaranteed to be selected prior to this method
        // if there's a lot of tiles selected then hide some options
        let is_multiple_tiles_selected = self.instances[instance_index].selected.len() > 1;

        // selected[0] will always hold because before this method call, we add to selected
        let effective_tile_index = self.instances[instance_index].selected[0];
        let effective_tile =
            &mut self.instances[instance_index].texture_tiles[effective_tile_index];

        // if there is ONE selected tile, then have everything from that tile instead
        // if clicked then copy the name of the texture

        if !is_multiple_tiles_selected {
            if ui
                .add(
                    egui::Label::new(effective_tile.name())
                        .selectable(false)
                        .sense(Sense::click()),
                )
                .on_hover_text("Click to copy name")
                .clicked()
            {
                ui.output_mut(|o| o.copied_text = effective_tile.name().to_string());
                ui.close_menu();
            }

            ui.separator();

            if ui.button("View").clicked() {
                self.extra_image_viewports
                    .push(effective_tile.wad_image.clone());
                ui.close_menu();
            }

            ui.separator();

            if ui.button("Rename").clicked() {
                effective_tile.in_rename = true;
                context_menu_clicked = true;

                effective_tile
                    .prev_name
                    .clone_from(&effective_tile.name().clone());
                ui.close_menu();
            }
        }

        // export when there's lots of selected or not
        if !is_multiple_tiles_selected {
            if ui.button("Export").clicked() {
                #[cfg(target_arch = "x86_64")]
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(effective_tile.name())
                    .add_filter("All Files", &["bmp"])
                    .save_file()
                {
                    // tODO TOAST
                    if let Err(err) = self.instances[instance_index]
                        .waddy
                        .dump_texture_to_file(effective_tile_index, path)
                    {
                        println!("{}", err);
                    }
                }

                ui.close_menu();
            }
        } else {
            // when lots of selected there will be option to deselect
            if ui.button("Deselect all").clicked() {
                self.instances[instance_index].selected.clear();
                ui.close_menu();
            }

            if ui
                .button(format!(
                    "Export ({})",
                    self.instances[instance_index].selected.len()
                ))
                .clicked()
            {
                #[cfg(target_arch = "x86_64")]
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.instances[instance_index].selected.par_iter().for_each(
                        |&texture_tile_index| {
                            let current_tile =
                                &self.instances[instance_index].texture_tiles[texture_tile_index];

                            let texture_file_name = &current_tile.name();

                            // tODO TOAST
                            if let Err(err) =
                                self.instances[instance_index].waddy.dump_texture_to_file(
                                    texture_tile_index,
                                    path.join(texture_file_name),
                                )
                            {
                                println!("{}", err);
                            };
                        },
                    );
                }

                ui.close_menu();
            }
        }

        // "copy to" would copy the textures(s) to other instances
        // or texture (singular) to the clipboard
        ui.separator();
        ui.menu_button("Copy to", |ui| {
            if ui.button("Clipboard").clicked() {
                let image =
                    &self.instances[instance_index].waddy.wad().entries[effective_tile_index];

                let is_transparent = self.instances[instance_index].waddy.wad().entries
                    [effective_tile_index]
                    .directory_entry
                    .texture_name
                    .get_string()
                    .starts_with("{");

                #[cfg(target_arch = "x86_64")]
                {
                    use arboard::Clipboard;

                    if let Ok(mut clipboard) = Clipboard::new() {
                        clipboard
                            .set_image(arboard::ImageData {
                                width: image.file_entry.dimensions().0 as usize,
                                height: image.file_entry.dimensions().1 as usize,
                                bytes: image
                                    .file_entry
                                    .image()
                                    .iter()
                                    .flat_map(|&color_idx| {
                                        let [r, g, b] =
                                            image.file_entry.palette()[color_idx as usize];

                                        if color_idx == 255 && is_transparent {
                                            [r, g, b, 0]
                                        } else {
                                            [r, g, b, 255]
                                        }
                                    })
                                    .collect::<Vec<u8>>()
                                    .into(),
                            })
                            .unwrap();
                    }
                }

                ui.close_menu();
            }

            // very fucky rust borrow checker shit so it is like this way
            let instance_to_add_idx = self
                .instances
                .iter()
                .enumerate()
                // .filter(|(idx, _)| *idx != instance_index) // allow copy to self
                .fold(None, |acc, (idx, instance)| {
                    if ui
                        .button(
                            instance
                                .path
                                .as_ref()
                                .unwrap()
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap(),
                        )
                        .clicked()
                    {
                        ui.close_menu();
                        Some(idx)
                    } else {
                        acc
                    }
                });

            if let Some(instance_to_add_idx) = instance_to_add_idx {
                let to_add = if self.instances[instance_index].selected.is_empty() {
                    vec![
                        self.instances[instance_index].waddy.wad().entries[effective_tile_index]
                            .clone(),
                    ]
                } else {
                    self.instances[instance_index]
                        .selected
                        .iter()
                        .map(|&tile_idx| {
                            self.instances[instance_index].waddy.wad().entries[tile_idx].clone()
                        })
                        .collect()
                };

                // manually add seems very sad
                to_add.into_iter().for_each(|new_entry| {
                    self.instances[instance_to_add_idx]
                        .waddy
                        .wad_mut()
                        .entries
                        .push(new_entry);

                    // update num_dirs
                    // TODO don't do this and have the writer write the numbers for us
                    self.instances[instance_to_add_idx]
                        .waddy
                        .wad_mut()
                        .header
                        .num_dirs += 1;

                    self.update_after_add_image(ui, instance_to_add_idx);
                });
            }
        });

        ui.separator();

        // delete when lots of selected or not
        if !is_multiple_tiles_selected {
            if ui.button("Delete").clicked() {
                self.instances[instance_index]
                    .to_delete
                    .push(effective_tile_index);

                ui.close_menu();
            }
        } else if ui
            .button(format!(
                "Delete ({})",
                self.instances[instance_index].selected.len()
            ))
            .clicked()
        {
            let mut to_deletes = self.instances[instance_index].selected.clone();
            self.instances[instance_index]
                .to_delete
                .append(&mut to_deletes);

            ui.close_menu()
        }

        context_menu_clicked
    }

    // highlight 1 tile based on some actions and clear all of selected for that instance
    fn select_texture_tile(&mut self, instance_index: usize, texture_tile_index: usize) {
        self.instances[instance_index].selected.clear();
        self.instances[instance_index]
            .selected
            .push(texture_tile_index);
    }

    /// Returns index of texture to delete
    fn texture_tile(
        &mut self,
        ui: &mut Ui,
        instance_index: usize,
        texture_tile_index: usize,
        image_tile_size: f32,
    ) {
        // FIXME: reduce ram usage by at least 4 times
        let current_id = egui::Id::new(format!(
            "{}{}",
            self.instances[instance_index].texture_tiles[texture_tile_index].index,
            self.instances[instance_index].texture_tiles[texture_tile_index].name()
        ));

        let is_selected = self.instances[instance_index]
            .selected
            .contains(&texture_tile_index);
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
                let texture = self.instances[instance_index].texture_tiles[texture_tile_index]
                    .wad_image
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

                // TODO make this context menu only once instead of doing for every tiles
                // that means we might have to do something about the behavior of not clicking on any tiles
                let mut context_menu_clicked = false;

                clickable_image.context_menu(|ui| {
                    // select tile if right click and there is only 1 selected tile
                    if self.instances[instance_index].selected.len() <= 1 {
                        self.select_texture_tile(instance_index, texture_tile_index);
                    }

                    context_menu_clicked = self.texture_tile_context_menu(ui, instance_index);
                });

                // if left clicked, add to the list of selected
                if clickable_image.clicked() {
                    let input_modifiers = ui.input(|i| i.modifiers);

                    if input_modifiers.ctrl {
                        if is_selected {
                            let unselect_idx = self.instances[instance_index]
                                .selected
                                .iter()
                                .position(|&idx| idx == texture_tile_index)
                                .unwrap();

                            self.instances[instance_index].selected.remove(unselect_idx);
                        } else {
                            self.instances[instance_index]
                                .selected
                                .push(texture_tile_index);
                        }
                    } else if input_modifiers.shift {
                        if let Some(&last_index) = self.instances[instance_index].selected.last() {
                            let range_start = last_index.min(texture_tile_index);
                            let range_end = last_index.max(texture_tile_index);

                            for idx in range_start..=range_end {
                                if !self.instances[instance_index].selected.contains(&idx) {
                                    self.instances[instance_index].selected.push(idx);
                                }
                            }
                        } else {
                            self.instances[instance_index]
                                .selected
                                .push(texture_tile_index);
                        }
                    } else {
                        self.select_texture_tile(instance_index, texture_tile_index);
                    }
                }

                // middle click wound bring a new viewport
                if clickable_image.middle_clicked() {
                    self.extra_image_viewports.push(
                        self.instances[instance_index].texture_tiles[texture_tile_index]
                            .wad_image
                            .clone(),
                    );
                };

                ui.end_row();

                if self.instances[instance_index].texture_tiles[texture_tile_index].in_rename {
                    let widget = ui.add(
                        egui::TextEdit::singleline(
                            self.instances[instance_index].texture_tiles[texture_tile_index]
                                .name_mut(),
                        )
                        .font(egui::TextStyle::Small),
                    );

                    widget.request_focus();

                    if ui.input(|i| i.key_pressed(egui::Key::Escape))
                                || (widget.clicked_elsewhere() && !context_menu_clicked) // does not work because rename is clicked on the same tick
                                || widget.lost_focus()
                                || !widget.has_focus()
                    {
                        let current_tile =
                            &mut self.instances[instance_index].texture_tiles[texture_tile_index];

                        let prev_name = current_tile.prev_name.clone();

                        current_tile.in_rename = false;
                        current_tile.name_mut().clone_from(&prev_name);
                    } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let current_instance = &mut self.instances[instance_index];
                        let current_tile = &mut current_instance.texture_tiles[texture_tile_index];

                        // this is the only case where the name is changed successfully
                        current_tile.in_rename = false;

                        if let Err(err) = current_instance
                            .waddy
                            .rename_texture(texture_tile_index, current_tile.name().clone())
                        {
                            // TODO learn how to do toast
                            println!("{:?}", err);

                            let prev_name = current_tile.prev_name.clone();

                            current_tile.name_mut().clone_from(&prev_name);
                        } else if current_tile.name().len() >= 16 {
                            println!("Texture name is too long");

                            let prev_name = current_tile.prev_name.clone();

                            current_tile.name_mut().clone_from(&prev_name);
                        } else {
                            // this means things are good
                            self.instances[instance_index].is_changed = true;
                        }
                    }
                } else {
                    let current_instance = &mut self.instances[instance_index];
                    let current_tile = &mut current_instance.texture_tiles[texture_tile_index];

                    // beside the context menu, double click on the name would also enter rename mode
                    if ui
                        .label(custom_font(current_tile.name().clone()))
                        .double_clicked()
                    {
                        current_tile.in_rename = true;
                        current_tile
                            .prev_name
                            .clone_from(&current_tile.name().clone());

                        self.select_texture_tile(instance_index, texture_tile_index);
                    };
                }

                ui.end_row();
                ui.label(custom_font(format!(
                    "{}x{}",
                    self.instances[instance_index].texture_tiles[texture_tile_index]
                        .dimensions()
                        .0,
                    self.instances[instance_index].texture_tiles[texture_tile_index]
                        .dimensions()
                        .1
                )));
            });
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
        let tile_count = self.instances[instance_index].texture_tiles.len();

        let image_tile_size =
            BASE_IMAGE_TILE_SIZE * ui.ctx().options(|options| options.zoom_factor);
        let texture_per_row = ((ui.min_size().x / image_tile_size).floor() as usize).max(4);
        let row_height = 2. // margin
            + 18. * 2. // 2 labels
            + image_tile_size;

        let is_search_enabled = self.instances[instance_index].search.enable;
        let search_text = self.instances[instance_index].search.text.to_lowercase();
        let filtered_tiles = (0..tile_count)
            .filter(|&texture_tile| {
                if is_search_enabled {
                    self.instances[instance_index].texture_tiles[texture_tile]
                        .name()
                        .to_lowercase()
                        .contains(search_text.as_str())
                } else {
                    true
                }
            })
            .collect::<Vec<usize>>();

        let total_rows = filtered_tiles.len().div_ceil(texture_per_row);

        ScrollArea::vertical().drag_to_scroll(false).show_rows(
            ui,
            row_height,
            total_rows,
            |ui, row_range| {
                // each row is one grid of grids
                row_range.for_each(|row| {
                    egui::Grid::new(format!("waddy_grid_row{}", row))
                        .num_columns(texture_per_row)
                        .spacing([2., 2.])
                        .show(ui, |ui| {
                            filtered_tiles
                                .chunks(texture_per_row)
                                .nth(row)
                                .expect("invalid row")
                                .iter()
                                .for_each(|&texture_tile_index| {
                                    self.texture_tile(
                                        ui,
                                        instance_index,
                                        texture_tile_index,
                                        image_tile_size,
                                    );
                                });
                        });
                });
            },
        );
    }

    // gui when there's WAD loaded
    fn instance_ui(&mut self, ui: &mut Ui, instance_index: usize) {
        // should close to short-circuit the GUI and avoid accessing non existing info
        let mut should_close = false;

        ui.separator();

        ui.horizontal(|ui| {
            if self.instance_menu(ui, instance_index) {
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

        // search bar
        if self.instances[instance_index].search.enable {
            egui::TopBottomPanel::bottom(format!("search_bar{}", instance_index)).show(
                ui.ctx(),
                |ui| {
                    ui.horizontal(|ui| {
                        let text_edit = egui::TextEdit::singleline(
                            &mut self.instances[instance_index].search.text,
                        )
                        .hint_text("Search for texture");

                        let text_edit = ui.add(text_edit);

                        if self.instances[instance_index].search.should_focus {
                            text_edit.request_focus();
                            self.instances[instance_index].search.should_focus = false;
                        }

                        self.instances[instance_index].search.has_focus = text_edit.has_focus();
                    });
                },
            );
        }

        // Save WAD file with Ctrl+S
        ui.input(|i| {
            if i.modifiers.matches_exact(Modifiers::CTRL) && i.key_released(egui::Key::S) {
                self.menu_save(instance_index);
            }
        });

        // Pasting an image from clipboard with Ctrl+V
        let should_add_pasted_image = ui
            .input(|i| i.modifiers.matches_exact(Modifiers::CTRL) && i.key_released(egui::Key::V));

        #[cfg(target_arch = "x86_64")]
        if should_add_pasted_image {
            use arboard::Clipboard;

            if let Ok(mut clipboard) = Clipboard::new() {
                if let Ok(image) = clipboard.get_image() {
                    let rgba_image: RgbaImage = ImageBuffer::from_raw(
                        image.width as u32,
                        image.height as u32,
                        image.bytes.into_owned(),
                    )
                    .unwrap();

                    self.instances[instance_index]
                        .waddy
                        .add_texture_from_rgba_image("pasted_texture", rgba_image)
                        .unwrap();
                    self.update_after_add_image(ui, instance_index);
                } else if let Ok(uri) = clipboard.get_text() {
                    if uri.starts_with("file://") {
                        if let Ok(image) = image::open(uri.replace("file://", "")) {
                            let rgba_image = image.into_rgba8();

                            self.instances[instance_index]
                                .waddy
                                .add_texture_from_rgba_image("pasted_texture", rgba_image)
                                .unwrap();
                            self.update_after_add_image(ui, instance_index);
                        }
                    }
                }
            }
        }

        // CTRL+F to enable search bar
        // if search bar is enabled and not focused, CTRL+F will refocus search bar
        // otherwise, disable search bar
        ui.input(|i| {
            if i.modifiers.matches_exact(Modifiers::CTRL) && i.key_released(egui::Key::F) {
                if self.instances[instance_index].search.enable {
                    // if search bar is enabled, refocus if not focus
                    // if is focused then disable it
                    if self.instances[instance_index].search.has_focus {
                        self.instances[instance_index].search.enable = false;
                        self.instances[instance_index].search.text.clear();
                    } else {
                        self.instances[instance_index].search.should_focus = true;
                    }
                } else {
                    // if search bar is not enabled, just enable it
                    self.instances[instance_index].search.enable = true;
                    self.instances[instance_index].search.text.clear();
                    self.instances[instance_index].search.should_focus = true;
                }
            }
        });

        let is_escape_pressed = ui.input(|i| i.key_released(egui::Key::Escape));

        // ESC to clear selected and close menu
        // if search bar is enabled, don't clear selected yet
        if is_escape_pressed && !self.instances[instance_index].search.enable {
            ui.close_menu();
            self.instances[instance_index].selected.clear();
        }

        // ESC to close search bar
        // search bar would be the first one to get closed if ESC is pressed
        // if there's selected textures, it won't deselect them if there's seach bar enabled
        if is_escape_pressed {
            self.instances[instance_index].search.enable = false;
            self.instances[instance_index].search.text.clear();
        }

        // DEL to delete texture(s)
        // This only works when there's selected texture so...
        ui.input(|i| {
            if i.key_released(egui::Key::Delete) {
                let mut to_delete = self.instances[instance_index].selected.clone();

                self.instances[instance_index]
                    .to_delete
                    .append(&mut to_delete)
            }
        });

        // Delete textures if there's any
        if !self.instances[instance_index].to_delete.is_empty() {
            let mut to_delete = self.instances[instance_index].to_delete.clone();
            to_delete.sort();

            to_delete.iter().rev().for_each(|&delete| {
                self.instances[instance_index].texture_tiles.remove(delete);
                self.instances[instance_index].waddy.remove_texture(delete);
            });

            self.instances[instance_index].to_delete.clear();
            self.instances[instance_index].selected.clear();
            self.instances[instance_index].is_changed = true;
        }

        let ctx = ui.ctx();

        self.display_image_viewports(ctx);

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
                    if ext == "wad" || ext == "bsp" {
                        if let Err(err) = self.start_waddy_instance(ui, Some(path)) {
                            // TODO TOAST
                            println!("{}", err);
                        }
                    // if an image file is dropped, we will add that to the current wad file
                    } else if SUPPORTED_TEXTURE_FORMATS.contains(&ext.to_str().unwrap()) {
                        if let Err(err) = self.instances[instance_index]
                            .waddy
                            .add_texture_from_path(path)
                        {
                            println!("{}", err);
                        } else {
                            self.update_after_add_image(ui, instance_index);
                        }
                    }
                }
            }
        }
    }

    // call it right after adding ONE image to the underlying WAD file to add new tile
    fn update_after_add_image(&mut self, ui: &mut Ui, instance_index: usize) {
        // after adding a new texture, we have to update the gui to include that new file
        let new_entry = self.instances[instance_index]
            .waddy
            .wad()
            .entries
            .last()
            .unwrap();

        let texture_name = new_entry.directory_entry.texture_name.get_string();
        let dimensions = if let FileEntry::MipTex(miptex) = &new_entry.file_entry {
            (miptex.width, miptex.height)
        } else {
            unreachable!()
        };
        let wad_image = if let FileEntry::MipTex(miptex) = &new_entry.file_entry {
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
            .push(TextureTile::new(instance_index, wad_image));

        self.instances[instance_index].is_changed = true;
    }

    // FIXME: it is ram guzzler
    fn start_waddy_instance(&mut self, ui: &mut Ui, path: Option<&Path>) -> eyre::Result<()> {
        // return is_changed here so that the user knows they need to save the file to have the file
        let (waddy, is_changed) = if let Some(path) = path {
            let ext = path.extension().unwrap();

            if ext == "wad" {
                (Waddy::from_wad_file(path)?, false)
            } else if ext == "bsp" {
                (Waddy::from_bsp_file(path)?, true)
            } else {
                unreachable!()
            }
        } else {
            (Waddy::new(), true)
        };

        if !is_changed {
            // this only happens when we open a wad on disk rather than a new wad or bsp
            self.persistent_storage
                .lock()
                .unwrap()
                .push_waddy_recent_wads(path.unwrap().to_str().unwrap())
                .expect(PERSISTENT_STORAGE_RECENTLY_USED_UPDATE_ERROR);
        }

        let texture_tiles = waddy
            .wad()
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if let FileEntry::MipTex(miptex) = &entry.file_entry {
                    let wad_image = WadImage::from_wad_image(
                        ui,
                        entry.directory_entry.texture_name.get_string(),
                        miptex.mip_images[0].data.get_bytes(),
                        miptex.palette.get_bytes(),
                        (miptex.width, miptex.height),
                    );

                    return Some(TextureTile::new(index, wad_image));
                    // None
                }

                None
            })
            .collect::<Vec<TextureTile>>();

        self.instances.push(WaddyInstance {
            path: path.map(|path| path.with_extension("wad")),
            waddy,
            texture_tiles,
            is_changed,
            selected: vec![],
            to_delete: vec![],
            search: SearchBar::default(),
        });

        Ok(())
    }

    fn menu_open(&mut self, ui: &mut Ui) -> bool {
        #[cfg(target_arch = "x86_64")]
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            let ext = path.extension().unwrap();

            if ext == "wad" || ext == "bsp" {
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

    fn instance_menu(&mut self, ui: &mut Ui, instance_index: usize) -> bool {
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

            self.open_recent_menu_button(ui);

            ui.separator();

            if ui.button("Save (Ctrl+S)").clicked() {
                self.menu_save(instance_index);

                ui.close_menu();
            }

            if ui.button("Save As").clicked() {
                self.menu_save_as_dialogue(instance_index);

                ui.close_menu();
            }

            ui.separator();

            if ui.button("Find (Ctrl+F)").clicked() {
                if self.instances[instance_index].search.enable {
                    self.instances[instance_index].search.enable = false;
                } else {
                    self.instances[instance_index].search.enable = true;
                    self.instances[instance_index].search.should_focus = true;
                }

                self.instances[instance_index].search.text.clear();
                ui.close_menu();
            }

            ui.separator();

            if ui.button("Import").clicked() {
                // TODO this is not consistent with drag and drop behavior
                // this does not filter out file extension
                #[cfg(target_arch = "x86_64")]
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    if let Err(err) = self.instances[instance_index]
                        .waddy
                        .add_texture_from_path(path)
                    {
                        println!("{}", err);
                    } else {
                        self.update_after_add_image(ui, instance_index);
                    }
                }

                ui.close_menu();
            }

            if ui.button("Export All").clicked() {
                #[cfg(target_arch = "x86_64")]
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

                if ui.button("To UPPERCASE").clicked() {
                    (0..self.instances[instance_index].texture_tiles.len()).for_each(|tile_idx| {
                        let tile_name =
                            self.instances[instance_index].texture_tiles[tile_idx].name_mut();
                        let new_name = tile_name.to_uppercase();

                        tile_name.clone_from(&new_name);
                        self.instances[instance_index]
                            .waddy
                            .rename_texture(tile_idx, &new_name)
                            .expect("cannot rename texture");

                        self.instances[instance_index].is_changed = true;
                        ui.close_menu();
                    });
                }

                if ui.button("To lowercase").clicked() {
                    (0..self.instances[instance_index].texture_tiles.len()).for_each(|tile_idx| {
                        let tile_name =
                            self.instances[instance_index].texture_tiles[tile_idx].name_mut();
                        let new_name = tile_name.to_lowercase();

                        tile_name.clone_from(&new_name);
                        self.instances[instance_index]
                            .waddy
                            .rename_texture(tile_idx, &new_name)
                            .expect("cannot rename texture");

                        self.instances[instance_index].is_changed = true;
                        ui.close_menu();
                    });
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
            self.persistent_storage
                .lock()
                .unwrap()
                .push_waddy_recent_wads(path.to_str().unwrap())
                .expect(PERSISTENT_STORAGE_RECENTLY_USED_UPDATE_ERROR);

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
        #[cfg(target_arch = "x86_64")]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("All Files", &["wad"])
            .set_file_name(if let Some(path) = &self.instances[instance_index].path {
                path.file_stem().unwrap().to_str().unwrap()
            } else {
                ""
            })
            .save_file()
        {
            self.persistent_storage
                .lock()
                .unwrap()
                .push_waddy_recent_wads(path.to_str().unwrap())
                .expect(PERSISTENT_STORAGE_RECENTLY_USED_UPDATE_ERROR);

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

            self.open_recent_menu_button(ui);
        });

        ui.separator();
        ui.label("Drag and drop a WAD file to start.\nYou can also drop a BSP file if you want.");

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
                    if ext == "wad" || ext == "bsp" {
                        if let Err(err) = self.start_waddy_instance(ui, Some(path)) {
                            // TODO TOAST
                            println!("{}", err);
                        }
                    }
                }
            }
        }
    }

    fn open_recent_menu_button(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Open Recent", |ui| {
            let mutex = self.persistent_storage.clone();
            let persistent_storage = mutex.lock().unwrap();
            let recent_wads = persistent_storage.get_waddy_recent_wads();

            let to_remove = if recent_wads.is_none() || recent_wads.unwrap().is_empty() {
                ui.add_enabled(false, egui::Button::new("No recently opened"));

                None
            } else {
                let recent_wads = recent_wads.unwrap().to_owned();

                // start_waddy_instance will block until it has persistent_storage guard
                drop(persistent_storage);

                recent_wads.into_iter().find(|recent_wad| {
                    if ui.button(recent_wad.as_str()).clicked() {
                        let path = Path::new(recent_wad.as_str());

                        if path.exists() {
                            self.start_waddy_instance(ui, Some(Path::new(recent_wad.as_str())))
                                .expect("cannot start a Waddy instance");

                            ui.close_menu();
                        } else {
                            return true;
                        }
                    }

                    false
                })
            };

            if let Some(to_remove) = to_remove {
                mutex
                    .lock()
                    .unwrap()
                    .remove_waddy_recent_wads(&to_remove)
                    .expect(PERSISTENT_STORAGE_RECENTLY_USED_UPDATE_ERROR);
            }
        });
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
