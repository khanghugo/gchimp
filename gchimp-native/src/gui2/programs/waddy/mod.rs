use std::{
    collections::HashSet,
    path::PathBuf,
    time::{Duration, Instant},
};

use gchimp::utils::img_stuffs::generate_mipmaps_from_rgba_image;
use iced::{
    widget::{column, container, scrollable, text_input, Stack},
    Element, Length, Subscription, Task,
};

use image::RgbaImage;
use wad::types::Wad;

use crate::gui2::{
    constants::DEFAULT_DIMENSIONS,
    programs::waddy::{
        context_menu::{ContextMenu, ContextMenuMessage},
        menu_bar::MenuBar,
        tile::{get_texture_tiles_from_wad, TileMessage, WaddyTile},
    },
    utils::IMAGE_FORMATS,
    TabProgram,
};

mod context_menu;
mod menu_bar;
mod tile;

const DOUBLE_CLICK_TIME: u64 = 200; // in msec

#[derive(Default)]
struct WaddyGridSelect {
    last: Option<usize>,
    // in selecting range, this is the first tile to selected all of other tiles
    anchor: Option<usize>,
    tiles: HashSet<usize>,
}

#[derive(Default)]
struct WaddyGridEdit {
    tile: Option<usize>,
    prev_label: String,
    should_focus: bool,
}

pub struct WaddyGrid {
    tiles: Vec<WaddyTile>,
    selected: WaddyGridSelect,
    edit: WaddyGridEdit,
}

impl WaddyGrid {
    fn new(texture_tiles: Vec<WaddyTile>) -> Self {
        Self {
            tiles: texture_tiles,
            selected: WaddyGridSelect::default(),
            edit: WaddyGridEdit::default(),
        }
    }
}

pub struct WaddyProgram {
    id: iced::widget::text_input::Id,
    grid: WaddyGrid,
    // other info
    window_size: Option<iced::Size>,
    last_cursor_pos: Option<iced::Point>,
    modifiers: iced::keyboard::Modifiers,
    last_click_time: Option<Instant>,
    // searching
    is_search_enabled: bool,
    search_text: String,
    // context menu
    context_menu: ContextMenu,
    // waddy
    wad_path: Option<String>,
    menu_bar: MenuBar,
}

impl Default for WaddyProgram {
    fn default() -> Self {
        // let images = Wad::from_file("/home/khang/map_compiler/colors.wad")
        //     .unwrap()
        //     .entries
        //     .iter()
        //     .map(|entry| {
        //         let wad::types::FileEntry::MipTex(ref image) = entry.file_entry else {
        //             panic!()
        //         };

        //         let (pixels, (width, height)) = image.to_rgba();

        //         iced::widget::image::Handle::from_rgba(width, height, pixels)
        //     })
        //     .collect::<Vec<_>>();

        let texture_tiles = get_texture_tiles_from_wad(
            Wad::from_file("/home/khang/map_compiler/colors.wad").unwrap(),
        );

        let grid = WaddyGrid::new(texture_tiles);

        Self {
            id: iced::widget::text_input::Id::unique(),
            // _wad_images: images,
            grid,
            window_size: Some(DEFAULT_DIMENSIONS.into()),
            modifiers: Default::default(),
            is_search_enabled: false,
            search_text: "".into(),
            last_cursor_pos: None,
            context_menu: Default::default(),
            last_click_time: None,
            wad_path: None,
            menu_bar: Default::default(),
        }
    }
}

impl TabProgram for WaddyProgram {
    fn title(&self) -> &'static str {
        "Waddy"
    }

    fn program(&self) -> crate::gui2::Program {
        crate::gui2::Program::Waddy
    }
}

#[derive(Debug, Clone)]
pub enum WaddyMessage {
    None,
    // related to iced
    GetWindowSize,
    SetWindowSize(iced::Size),

    // related to tiles interaction
    TileMessage(usize, TileMessage),
    SelectLogic(usize),
    SelectTile(usize),
    SelectAllTiles,
    DeselectTile(usize),
    ClearSelected,

    // related to inputs
    UpdateModifier(iced::keyboard::Modifiers),
    LeftClick(iced::event::Status),
    RightClick(iced::event::Status),
    DoubleClick(iced::event::Status),
    UpdateCursorPos(iced::Point),
    UpdateRightClickPos,
    EscapePressed,

    // related to search
    SearchToggle,
    SearchText(String),

    // related to context menu
    ContextMenuToggle(bool),

    // related to waddy
    WadSave,
    WadSaved(Option<String>),
    FileDropped(PathBuf),
    TextureAddRequest(PathBuf),
    TextureAdd((String, RgbaImage)),

    // debug
    Debug(String),
}

impl WaddyProgram {
    pub fn view(&'_ self) -> Element<'_, WaddyMessage> {
        let tile_grid = iced::widget::row(
            self.grid
                .tiles
                .iter()
                .enumerate()
                .filter(|(_, tile)| {
                    if self.is_search_enabled {
                        tile.label.contains(self.search_text.as_str())
                    } else {
                        true
                    }
                })
                .map(|(idx, tile)| {
                    let is_selected = self.grid.selected.tiles.contains(&idx);
                    let is_in_edit = self.grid.edit.tile.map(|x| x == idx).unwrap_or(false);

                    tile.view(is_selected, is_in_edit)
                        .map(move |message| WaddyMessage::TileMessage(idx, message))
                }),
        )
        .width(Length::Fill)
        .spacing(iced::Pixels(4.))
        .wrap();

        let space_rest = || iced::widget::Space::with_height(Length::Fill);

        let tile_browser = scrollable(tile_grid);

        let menu_bar = self.menu_bar.view();
        // need to pad the space for some reasons
        let browser = column![menu_bar, tile_browser, space_rest()];

        let search_bar = container(
            text_input("search for texture", &self.search_text)
                .id(self.id.clone())
                // .on_input_maybe(on_input)
                .on_input(WaddyMessage::SearchText)
                .style(|theme: &iced::Theme, _status| {
                    // default style but no border
                    let palette = theme.extended_palette();

                    iced::widget::text_input::Style {
                        background: iced::Background::Color(palette.background.base.color),
                        border: iced::Border {
                            radius: 2.into(),
                            width: 1.,
                            color: palette.background.strong.color,
                        },
                        icon: palette.background.weak.text,
                        placeholder: palette.background.strong.color,
                        value: palette.background.base.text,
                        selection: palette.primary.weak.color,
                    }
                })
                .width(Length::Fixed(200.)),
        )
        .padding(4);

        let search_bar = container(column![space_rest(), search_bar]);

        // let waddy_col = if self.is_search_enabled {
        //     waddy_col.push(search_bar)
        // } else {
        //     waddy_col
        // };
        let stack = Stack::new().push(browser);

        let stack = if self.is_search_enabled {
            stack.push(search_bar)
        } else {
            stack
        };

        // texture name to appear when open context menu
        let texture_name = if self.grid.selected.tiles.len() == 1 {
            self.grid
                .selected
                .tiles
                .iter()
                .next()
                .and_then(|&tile| self.grid.tiles.get(tile))
                .map(|tile| tile.label.clone())
        } else {
            None
        };

        let context_menu = self.context_menu.view(
            self.last_cursor_pos,
            self.window_size,
            texture_name,
            self.grid.selected.last,
        );

        let stack = stack.push(context_menu);

        stack.into()
    }

    pub fn update(&mut self, message: WaddyMessage) -> Task<WaddyMessage> {
        // focus on the next frame
        // TODO maybe there is a better way to do this
        if self.grid.edit.should_focus {
            self.grid.edit.should_focus = false;

            let idx = self.grid.edit.tile.unwrap();

            return iced::widget::text_input::focus(self.grid.tiles[idx].id.clone());
        }

        // in the future we might need to chain with this. whatever.
        // TODO maybe chain this
        match message {
            WaddyMessage::None => {}
            WaddyMessage::GetWindowSize => {
                return iced::window::get_latest()
                    .and_then(iced::window::get_size)
                    .map(WaddyMessage::SetWindowSize);
            }
            WaddyMessage::SetWindowSize(size) => {
                self.window_size = size.into();
            }
            WaddyMessage::TileMessage(idx, TileMessage::RightClick) => {
                let _ = self.update(WaddyMessage::UpdateRightClickPos);

                // if self.context_menu.is_enable {
                //     // if context menu is enabled then just simply close it
                //     let _ = self.update(WaddyMessage::ContextMenuToggle(false));
                // } else {
                //     // if context menu is not enabled, select that new tile
                //     // and then open context menu

                //     // if the tile is selected, then don't select new tile because selecting new tile
                //     // will reset selected tiles.
                //     if !self.selected_tiles().contains(&idx) {
                //         let _ = self.update(WaddyMessage::TileMessage(idx, TileMessage::Select));
                //     }

                //     let _ = self.update(WaddyMessage::ContextMenuToggle(true));
                // }

                if self.context_menu.is_enable {
                    let _ = self.update(WaddyMessage::ContextMenuToggle(false));
                } else {
                    let _ = self.update(WaddyMessage::SelectLogic(idx));
                    let _ = self.update(WaddyMessage::ContextMenuToggle(true));
                }
            }
            WaddyMessage::TileMessage(idx, TileMessage::LeftClick) => {
                // if there is any context menu, that means we cannot select the tile
                // so close the context menu and return
                if self.context_menu.is_enable {
                    let _ = self.update(WaddyMessage::ContextMenuToggle(false));

                    return Task::none();
                }

                let _ = self.update(WaddyMessage::SelectLogic(idx));
            }
            WaddyMessage::TileMessage(idx, TileMessage::EditRequest) => {
                // last_click_time is only concerned with editing the name so no need to have it
                // in bigger scope yet
                // TODO: maybe make self.last_click_time work properly in the application scope
                let now = Instant::now();

                // select the tile before doing anything else
                let _ = self.update(WaddyMessage::SelectLogic(idx));

                if let Some(last_click_time) = self.last_click_time {
                    let duration = now.duration_since(last_click_time);

                    // enter edit if double click fast enough
                    // and also clicking on the same tile
                    let clicked_fast = duration <= Duration::from_millis(DOUBLE_CLICK_TIME);
                    let clicked_on_same_tile = self.grid.selected.last.is_some_and(|x| x == idx);

                    if clicked_fast && clicked_on_same_tile {
                        let _ = self.update(WaddyMessage::TileMessage(idx, TileMessage::EditEnter));
                    }
                }

                self.last_click_time = now.into();

                // selecting title wont select the tile, so things are simpler
                // // disable context menu always
                // let _ = self.update(WaddyMessage::ContextMenuToggle(false));

                // // this counts as clicking something so select logic applies
                // let _ = self.update(WaddyMessage::SelectLogic(idx));
            }
            WaddyMessage::TileMessage(idx, TileMessage::EditEnter) => {
                let _ = self.update(WaddyMessage::ClearSelected);
                let _ = self.update(WaddyMessage::ContextMenuToggle(false));

                self.grid.edit.tile = Some(idx);

                let tile = &self.grid.tiles[idx];

                self.grid.edit.prev_label = tile.label.clone();

                // the element is not rendered yet to focus on
                // iced::widget::text_input::focus(tile_id)
                self.grid.edit.should_focus = true;
            }
            WaddyMessage::TileMessage(_, TileMessage::EditFinish) => {
                self.grid.edit.tile = None;
            }
            WaddyMessage::TileMessage(idx, TileMessage::EditCancel) => {
                let prev_labbel = self.grid.edit.prev_label.to_owned();

                // idx must be self.grid.in_edit
                assert_eq!(idx, self.grid.edit.tile.unwrap());

                self.grid.tiles.get_mut(idx).map(|x| {
                    x.label = prev_labbel;
                });

                self.grid.edit.tile = None;
            }
            WaddyMessage::TileMessage(idx, TileMessage::EditChange(text)) => {
                self.grid.tiles.get_mut(idx).map(|x| {
                    x.label = text;
                    x.label.truncate(15);
                });
            }
            WaddyMessage::SelectLogic(idx) => {
                // confirm edit before selecting tiles
                // tile number doesnt matter here
                let _ = self.update(WaddyMessage::TileMessage(0, TileMessage::EditFinish));

                if self.modifiers.control() {
                    // if current tile is selected and is ctrl seleted then unselect it
                    if self.grid.selected.tiles.contains(&idx) {
                        let _ = self.update(WaddyMessage::DeselectTile(idx));
                    } else {
                        let _ = self.update(WaddyMessage::SelectTile(idx));
                    }

                    // clear anchor because we want new range
                    self.grid.selected.anchor = None;
                } else if self.modifiers.shift() {
                    // selecting range, i want this to mimic kde dolphin behavior
                    // if an anchor is not known, last selected tile will be the anchor
                    // after that, all tiles from anchor and last selected will be selected
                    // there is no deselect but there will be a clear from anchor to last selected

                    if let Some(last_selected) = self.grid.selected.last {
                        // if anchor is not set, set it right away
                        let anchor = self
                            .grid
                            .selected
                            .anchor
                            .get_or_insert(last_selected)
                            .to_owned();

                        // deselect from anchor to last_selected
                        ((anchor.min(last_selected))..=(anchor.max(last_selected))).for_each(
                            |idx| {
                                let _ = self.update(WaddyMessage::DeselectTile(idx));
                            },
                        );

                        // select from anchor to idx
                        ((anchor.min(idx))..=(anchor.max(idx))).for_each(|idx| {
                            let _ = self.update(WaddyMessage::SelectTile(idx));
                        });
                    }
                } else {
                    // normal select is unselecting everything then select new one
                    let _ = self.update(WaddyMessage::ClearSelected);
                    let _ = self.update(WaddyMessage::SelectTile(idx));

                    // clear anchor just in case
                    self.grid.selected.anchor = None;
                }
            }
            WaddyMessage::SelectTile(idx) => {
                self.grid.selected.tiles.insert(idx);
                self.grid.selected.last = idx.into();
            }
            WaddyMessage::DeselectTile(idx) => {
                self.grid.selected.tiles.remove(&idx);
                self.grid.selected.last = None;
            }
            WaddyMessage::ClearSelected => {
                self.grid.selected.tiles.clear();
                self.grid.selected.last = None;
            }
            WaddyMessage::SelectAllTiles => {
                self.grid.selected.tiles = HashSet::from_iter(0..self.grid.tiles.len());
            }
            WaddyMessage::UpdateModifier(modifiers) => {
                self.modifiers = modifiers;
            }
            WaddyMessage::SearchToggle => {
                self.is_search_enabled = !self.is_search_enabled;

                if self.is_search_enabled {
                    // must clear search text for next search
                    self.search_text.clear();

                    return iced::widget::text_input::focus(self.id.clone());
                }
            }
            WaddyMessage::SearchText(string) => {
                self.search_text = string;
            }
            WaddyMessage::ContextMenuToggle(bool) => {
                self.context_menu.update(ContextMenuMessage::Toggle(bool));
            }
            WaddyMessage::LeftClick(status) => {
                // proceed with normal click
                if matches!(status, iced::event::Status::Captured) {
                    // this means we left click on a tile
                } else {
                    // if not captured means we click on somewhere empty
                    if self.context_menu.is_enable {
                        // if there is context menu, close it
                        let _ = self.update(WaddyMessage::ContextMenuToggle(false));
                    } else {
                        // otherwise, deselec everything
                        let _ = self.update(WaddyMessage::ClearSelected);
                    }
                }
            }
            WaddyMessage::RightClick(status) => {
                // right click where the click is not captured
                // this will attempt to toggle context menu
                // it will depend on the context menu to show stuffs if there is any tile selected or not
                if matches!(status, iced::event::Status::Captured) {
                    // if click is captured, it is up to the widget to handle it
                    // right click can selct context menu item so this will close it as well
                    // if self.context_menu.is_enable {
                    //     let _ = self.update(WaddyMessage::ContextMenuToggle(false));
                    // }
                } else {
                    let _ = self.update(WaddyMessage::ContextMenuToggle(
                        !self.context_menu.is_enable,
                    ));
                }

                let _ = self.update(WaddyMessage::UpdateRightClickPos);
            }
            WaddyMessage::DoubleClick(status) => {}
            WaddyMessage::Debug(msg) => {
                println!("{}", msg);
            }
            WaddyMessage::UpdateCursorPos(point) => {
                self.last_cursor_pos = point.into();
            }
            WaddyMessage::UpdateRightClickPos => {
                if let Some(last_cursor_pos) = self.last_cursor_pos {
                    self.context_menu
                        .update(ContextMenuMessage::UpdateLastRightClick(last_cursor_pos));
                }
            }
            WaddyMessage::WadSave => {
                #[cfg(not(target_arch = "wasm32"))]
                let wad_path = self.wad_path.clone().or(rfd::FileDialog::new()
                    .add_filter("WAD", &["wad"])
                    .save_file()
                    .map(|path| path.display().to_string()));

                let Some(wad_path) = wad_path else {
                    return Task::none();
                };

                let mut wad = Wad::new();

                let entries = self
                    .grid
                    .tiles
                    .iter()
                    .filter_map(|tile| {
                        let iced::widget::image::Handle::Rgba {
                            id: _id,
                            width,
                            height,
                            ref pixels,
                        } = tile.handle
                        else {
                            unreachable!("iced image is not rgba bytes")
                        };

                        let Some(image) =
                            image::RgbaImage::from_raw(width, height, pixels.to_vec())
                        else {
                            unreachable!("cannot convert iced rgba image to rgba image")
                        };

                        generate_mipmaps_from_rgba_image(image).ok()
                    })
                    .zip(self.grid.tiles.iter().map(|tile| tile.label.clone()))
                    .map(|(res, texture_name)| {
                        wad::types::Entry::new(
                            texture_name,
                            res.dimensions,
                            &[&res.mips[0], &res.mips[1], &res.mips[2], &res.mips[3]],
                            res.palette,
                        )
                    })
                    .collect::<Vec<_>>();

                if entries.len() != self.grid.tiles.len() {
                    // TODO toast component
                    println!("cannot convert all tiles to textures");

                    return Task::none();
                }

                entries.into_iter().for_each(|entry| {
                    wad.entries.push(entry);
                    wad.header.num_dirs += 1;
                });

                return Task::perform(async move { wad.write_to_file(wad_path) }, |res| {
                    println!("it is done writing");
                    WaddyMessage::WadSaved(res.err().map(|f| f.to_string()))
                });
            }
            WaddyMessage::WadSaved(err) => {
                if let Some(err) = err {
                    println!("cannot write wad file: {}", err);
                }
            }
            WaddyMessage::FileDropped(path_buf) => {
                let Some(ext) = path_buf.extension() else {
                    return Task::none();
                };

                const WAD_EXTENSION: &str = "wad";
                const BSP_EXTENSION: &str = "bsp";

                if ext == WAD_EXTENSION {
                    todo!()
                } else if ext == BSP_EXTENSION {
                    todo!()
                } else if IMAGE_FORMATS.iter().any(|&x| x == ext) {
                    return Task::done(WaddyMessage::TextureAddRequest(path_buf));
                } else {
                }
            }
            WaddyMessage::TextureAddRequest(path_buf) => {
                return Task::perform(
                    async move { image::open(path_buf.as_path()).map(|res| (path_buf, res.to_rgba8())) },
                    |res| match res {
                        Ok((path_buf, image)) => WaddyMessage::TextureAdd((
                            path_buf.file_stem().unwrap().to_str().unwrap().to_string(),
                            image,
                        )),
                        Err(err) => {
                            println!("cannot open image: {}", err);
                            WaddyMessage::None
                        }
                    },
                )
            }
            WaddyMessage::TextureAdd((texture_name, image_buffer)) => {
                let new_tile = WaddyTile {
                    id: iced::widget::text_input::Id::unique(),
                    label: texture_name,
                    dimensions: image_buffer.dimensions(),
                    handle: iced::widget::image::Handle::from_rgba(
                        image_buffer.width(),
                        image_buffer.height(),
                        image_buffer.into_vec(),
                    ),
                };

                self.grid.tiles.push(new_tile);
            }
            WaddyMessage::EscapePressed => {
                // if there is context menu, close it
                if self.context_menu.is_enable {
                    return Task::done(WaddyMessage::ContextMenuToggle(false));
                }

                // if there is search, close it
                if self.is_search_enabled {
                    return Task::done(WaddyMessage::SearchToggle);
                }

                // if there is a tile in edit, cancel edit
                if let Some(in_edit_tile) = self.grid.edit.tile {
                    let _ = self.update(WaddyMessage::TileMessage(
                        in_edit_tile,
                        TileMessage::EditCancel,
                    ));

                    return Task::none();
                }

                // otherwise, reset all tile states and deselect all
                // reset tile states
                let _ = self.update(WaddyMessage::ClearSelected);
            }
        };

        Task::none()
    }

    pub fn subscription(&self) -> Subscription<WaddyMessage> {
        // must use listen raw because we want to process the input throughout the program
        // instead of stopping at a widget
        // eg, while typing in search bar, we can ctrl f again to close it
        // with normal event::listen, text_input will capture every keys so it cannot be closed
        iced::event::listen_raw(|event, status, _id| match event {
            iced::Event::Window(event) => match event {
                iced::window::Event::FileDropped(path_buf) => {
                    WaddyMessage::FileDropped(path_buf).into()
                }
                iced::window::Event::Resized(size) => WaddyMessage::SetWindowSize(size).into(),
                _ => None,
            },
            iced::Event::Mouse(event) => match event {
                iced::mouse::Event::ButtonPressed(button) => match button {
                    iced::mouse::Button::Left => WaddyMessage::LeftClick(status).into(),
                    iced::mouse::Button::Right => WaddyMessage::RightClick(status).into(),
                    _ => None,
                },

                iced::mouse::Event::CursorMoved { position } => {
                    WaddyMessage::UpdateCursorPos(position).into()
                }
                _ => None,
            },
            iced::Event::Keyboard(event) => match event {
                iced::keyboard::Event::ModifiersChanged(modifiers) => {
                    WaddyMessage::UpdateModifier(modifiers).into()
                }
                iced::keyboard::Event::KeyPressed { key, modifiers, .. } => {
                    let is_ctrl_x_fn = |input| {
                        matches!(key.as_ref(),
                        // what the fuck is this?
                        iced::keyboard::Key::Character(s) if s == input)
                            && modifiers.control()
                    };

                    let is_ctrl_f = is_ctrl_x_fn("f");
                    let is_ctrl_a = is_ctrl_x_fn("a");
                    let is_ctrl_s = is_ctrl_x_fn("s");

                    if is_ctrl_a {
                        return WaddyMessage::SelectAllTiles.into();
                    }

                    if is_ctrl_f {
                        return WaddyMessage::SearchToggle.into();
                    }

                    if is_ctrl_s {
                        println!("it is ctrl s");
                        return WaddyMessage::WadSave.into();
                    }

                    if matches!(
                        key.as_ref(),
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
                    ) {
                        return WaddyMessage::EscapePressed.into();
                    }

                    None
                }
                _ => None,
            },
            _ => None,
        })
    }
}
