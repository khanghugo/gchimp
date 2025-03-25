use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use eyre::eyre;
use gchimp::utils::img_stuffs::generate_mipmaps_from_rgba_image;
use iced::{
    widget::{
        button, column, container, horizontal_rule, mouse_area, row, scrollable, text, text_input,
        vertical_rule, Stack,
    },
    Alignment, ContentFit, Element, Length, Padding, Subscription, Task,
};
use iced_aw::{menu::Item, menu_items};

use image::RgbaImage;
use wad::types::Wad;

use crate::{
    context_menu_button,
    gui2::{constants::DEFAULT_DIMENSIONS, utils::IMAGE_FORMATS, TabProgram},
    menu_labeled_button, menu_submenu_button, spaced_row,
};

const TILE_DIMENSION: f32 = 134.;
const DOUBLE_CLICK_TIME: u64 = 200; // in msec

pub struct WaddyProgram {
    id: iced::widget::text_input::Id,
    // tiles
    texture_tiles: Vec<TextureTile>,
    last_selected: Option<usize>,
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

        Self {
            id: iced::widget::text_input::Id::unique(),
            // _wad_images: images,
            texture_tiles,
            window_size: Some(DEFAULT_DIMENSIONS.into()),
            modifiers: Default::default(),
            is_search_enabled: false,
            search_text: "".into(),
            last_cursor_pos: None,
            context_menu: Default::default(),
            last_selected: None,
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

// impl From<WaddyMessage> for Option<WaddyMessage> {

// }

impl WaddyProgram {
    pub fn view(&self) -> Element<WaddyMessage> {
        let tile_grid = iced::widget::row(
            self.texture_tiles
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    if self.is_search_enabled {
                        e.name.contains(self.search_text.as_str())
                    } else {
                        true
                    }
                })
                .map(|(idx, e)| {
                    e.view()
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

        let selected_tiles = self.selected_tiles();

        let texture_name = if selected_tiles.len() == 1 {
            selected_tiles
                .first()
                .and_then(|&tile| self.texture_tiles.get(tile))
                .map(|tile| tile.name.clone())
        } else {
            None
        };

        let context_menu = self.context_menu.view(
            self.last_cursor_pos,
            self.window_size,
            texture_name,
            self.last_selected,
        );

        let stack = stack.push(context_menu);

        stack.into()
    }

    pub fn update(&mut self, message: WaddyMessage) -> Task<WaddyMessage> {
        match message {
            WaddyMessage::None => Task::none(),
            WaddyMessage::GetWindowSize => iced::window::get_latest()
                .and_then(iced::window::get_size)
                .map(WaddyMessage::SetWindowSize),
            WaddyMessage::SetWindowSize(size) => {
                self.window_size = size.into();

                Task::none()
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

                Task::none()
            }
            WaddyMessage::TileMessage(idx, TileMessage::LeftClick) => {
                // if there is any context menu, that means we cannot select the tile
                // so close the context menu and return
                if self.context_menu.is_enable {
                    let _ = self.update(WaddyMessage::ContextMenuToggle(false));

                    return Task::none();
                }

                let _ = self.update(WaddyMessage::SelectLogic(idx));

                return Task::none();
            }
            WaddyMessage::TileMessage(idx, TileMessage::EditRequest) => {
                // last_click_time is only concerned with editing the name so no need to have it
                // in bigger scope yet
                // TODO: maybe make self.last_click_time work properly in the application scope
                let now = Instant::now();

                if let Some(last_click_time) = self.last_click_time {
                    let duration = now.duration_since(last_click_time);

                    if duration <= Duration::from_millis(DOUBLE_CLICK_TIME) {
                        let _ = self.update(WaddyMessage::ClearSelected);
                        let _ = self.update(WaddyMessage::SelectLogic(idx));
                        let _ = self.update(WaddyMessage::TileMessage(idx, TileMessage::EditEnter));
                    }
                }

                self.last_click_time = now.into();

                // disable context menu always
                let _ = self.update(WaddyMessage::ContextMenuToggle(false));

                // this counts as clicking something so select logic applies
                let _ = self.update(WaddyMessage::SelectLogic(idx));

                Task::none()
            }
            WaddyMessage::TileMessage(idx, texture_tile_mesage) => {
                if let Some(tile) = self.texture_tiles.get_mut(idx) {
                    // intercepting the message to do more things
                    let task = match texture_tile_mesage {
                        TileMessage::EditEnter => {
                            self.context_menu.is_enable = false;

                            iced::widget::text_input::focus(tile.id.clone())
                        }
                        _ => Task::none(),
                    };

                    tile.update(texture_tile_mesage);

                    return task;
                };

                Task::none()
            }
            // TODO does this count as abusing?
            // this is like writing small methods but they are not methods
            WaddyMessage::SelectLogic(idx) => {
                if self.modifiers.control() {
                    // if current tile is selected and is ctrl seleted then unselect it
                    if self.selected_tiles().contains(&idx) {
                        let _ = self.update(WaddyMessage::DeselectTile(idx));
                    } else {
                        let _ = self.update(WaddyMessage::SelectTile(idx));
                    }
                } else if self.modifiers.shift() {
                    // selecting range
                    // range exclusive max because
                    // if max is idx, it will be selected outside of this scope
                    // if max is last selected, it is included
                    let selected_tiles = self.selected_tiles();

                    // TODO make good
                    if let Some(last_selected) = self.last_selected {
                        (idx.min(last_selected)..=idx.max(last_selected)).for_each(|idx| {
                            if selected_tiles.contains(&idx) {
                                let _ = self.update(WaddyMessage::DeselectTile(idx));
                            } else {
                                let _ = self.update(WaddyMessage::SelectTile(idx));
                            }
                        });

                        // keep last selected
                        self.last_selected = last_selected.into();
                    }

                    // then select the tile
                    let _ = self.update(WaddyMessage::SelectTile(idx));
                } else {
                    // normal select is unselecting everything then select new one
                    let _ = self.update(WaddyMessage::ClearSelected);
                    let _ = self.update(WaddyMessage::SelectTile(idx));
                }

                Task::none()
            }
            WaddyMessage::SelectTile(idx) => {
                if let Some(tile) = self.texture_tiles.get_mut(idx) {
                    tile.update(TileMessage::Select);

                    self.last_selected = idx.into()
                };

                Task::none()
            }
            WaddyMessage::DeselectTile(idx) => {
                if let Some(tile) = self.texture_tiles.get_mut(idx) {
                    tile.update(TileMessage::Deselect);

                    self.last_selected = self.selected_tiles().pop();
                };

                Task::none()
            }
            WaddyMessage::ClearSelected => {
                self.texture_tiles.iter_mut().for_each(|tile| {
                    tile.state = TileState::None;
                });

                Task::none()
            }
            WaddyMessage::SelectAllTiles => {
                self.texture_tiles.iter_mut().for_each(|tile| {
                    tile.state = TileState::Selected;
                });

                Task::none()
            }
            WaddyMessage::UpdateModifier(modifiers) => {
                self.modifiers = modifiers;

                Task::none()
            }
            WaddyMessage::SearchToggle => {
                self.is_search_enabled = !self.is_search_enabled;

                if self.is_search_enabled {
                    // must clear search text for next search
                    self.search_text.clear();

                    return iced::widget::text_input::focus(self.id.clone());
                }

                Task::none()
            }
            WaddyMessage::SearchText(string) => {
                self.search_text = string;

                Task::none()
            }
            WaddyMessage::ContextMenuToggle(bool) => {
                self.context_menu.update(ContextMenuMessage::Toggle(bool));

                Task::none()
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

                Task::none()
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

                Task::none()
            }
            WaddyMessage::DoubleClick(status) => Task::none(),
            WaddyMessage::Debug(msg) => {
                println!("{}", msg);

                Task::none()
            }
            WaddyMessage::UpdateCursorPos(point) => {
                self.last_cursor_pos = point.into();

                Task::none()
            }
            WaddyMessage::UpdateRightClickPos => {
                if let Some(last_cursor_pos) = self.last_cursor_pos {
                    self.context_menu
                        .update(ContextMenuMessage::UpdateLastRightClick(last_cursor_pos));
                }

                Task::none()
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
                    .texture_tiles
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
                    .zip(self.texture_tiles.iter().map(|tile| tile.name.clone()))
                    .map(|(res, texture_name)| {
                        wad::types::Entry::new(
                            texture_name,
                            res.dimensions,
                            &[&res.mips[0], &res.mips[1], &res.mips[2], &res.mips[3]],
                            res.palette,
                        )
                    })
                    .collect::<Vec<_>>();

                if entries.len() != self.texture_tiles.len() {
                    // TODO toast component
                    println!("cannot convert all tiles to textures");

                    return Task::none();
                }

                entries.into_iter().for_each(|entry| {
                    wad.entries.push(entry);
                    wad.header.num_dirs += 1;
                });

                Task::perform(async move { wad.write_to_file(wad_path) }, |res| {
                    println!("it is done writing");
                    WaddyMessage::WadSaved(res.err().map(|f| f.to_string()))
                })
            }
            WaddyMessage::WadSaved(err) => {
                if let Some(err) = err {
                    println!("cannot write wad file: {}", err);
                }

                Task::none()
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
                    Task::done(WaddyMessage::TextureAddRequest(path_buf))
                } else {
                    Task::none()
                }
            }
            WaddyMessage::TextureAddRequest(path_buf) => Task::perform(
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
            ),
            WaddyMessage::TextureAdd((texture_name, image_buffer)) => {
                let new_tile = TextureTile {
                    id: iced::widget::text_input::Id::unique(),
                    name: texture_name,
                    dimensions: image_buffer.dimensions(),
                    handle: iced::widget::image::Handle::from_rgba(
                        image_buffer.width(),
                        image_buffer.height(),
                        image_buffer.into_vec(),
                    ),
                    state: TileState::None,
                };

                self.texture_tiles.push(new_tile);

                Task::none()
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

                // if there is a tile in edit, set it back to selected
                let mut changed_edit_tile = false;

                self.texture_tiles
                    .iter_mut()
                    // there should be only 1 tiles in edit at a time
                    // so this will help catching unwanted behavior
                    .find(|tile| tile.state.is_in_edit())
                    .map(|tile| {
                        changed_edit_tile = true;
                        tile.state = TileState::Selected;
                    });

                if changed_edit_tile {
                    return Task::none();
                }

                // otherwise, reset all tile states and deselect all
                // reset tile states
                self.texture_tiles.iter_mut().for_each(|tile| {
                    tile.state = TileState::None;
                });

                let _ = self.update(WaddyMessage::ClearSelected);

                Task::none()
            }
        }
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

    fn selected_tiles(&self) -> Vec<usize> {
        self.texture_tiles
            .iter()
            .enumerate()
            .filter_map(|(idx, e)| {
                if e.state.is_selected() {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone)]
struct MenuBar {
    texture_fit: bool,
}

#[derive(Debug, Clone)]
enum MenuBarMessage {
    None,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self { texture_fit: true }
    }
}

impl MenuBar {
    fn view(&self) -> Element<WaddyMessage> {
        // let menu_tpl_1 = |items| iced_aw::Menu::new(items).max_width(180.0).offset(15.0).spacing(5.0);
        let menu_tpl_2 = |items| {
            iced_aw::Menu::new(items)
                .max_width(180.0)
                .offset(0.0)
                .spacing(5.0)
        };

        let button1 = menu_submenu_button!("hello8", WaddyMessage::None);

        let menu1 = menu_tpl_2(menu_items!((menu_labeled_button!(
            "aa",
            WaddyMessage::None
        ))(button("hello2"))(button("hello3"))));

        let menu2 = menu_tpl_2(menu_items!((button("hello4"))(button("hello5"))(button(
            "hello6"
        ))));

        let menu3 = menu_tpl_2(menu_items!((button("hello7"))(button1, menu2)(button(
            "hello9"
        ))));

        let menu = iced_aw::menu_bar!((button("text1"), menu1)(button("text2"), menu3));

        menu.into()
    }
}

struct ContextMenu {
    is_enable: bool,
    // last right click position
    last_right_click: Option<iced::Point>,
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self {
            is_enable: false,
            last_right_click: None,
        }
    }
}

enum ContextMenuMessage {
    None,
    Toggle(bool),
    UpdateLastRightClick(iced::Point<f32>),
}

impl ContextMenu {
    fn view(
        &self,
        last_cursor_pos: Option<iced::Point>,
        window_size: Option<iced::Size<f32>>,
        texture_name: Option<String>,
        last_selected: Option<usize>,
    ) -> Element<WaddyMessage> {
        if !self.is_enable {
            return column![].into();
        }

        // context menu is on top
        // the reason why this is a custom component with fuckery code is iced_aw
        // has a component for context menu but it will hijack the right click
        // i want right click to select the component as well
        // this means the component does not have a right click listener

        // i love iced
        // i love doing basic things like this over and over
        let (align_x, align_y, padding) = {
            let context_menu_pos = self.last_right_click.or(last_cursor_pos);

            if let Some(context_menu_pos) = context_menu_pos {
                const MIN_WIDTH: f32 = 80.;
                const MIN_HEIGHT: f32 = 160.;

                let align_x = if context_menu_pos.x + MIN_WIDTH > window_size.unwrap().width {
                    Alignment::End
                } else {
                    Alignment::Start
                };

                let align_y = if context_menu_pos.y + MIN_HEIGHT > window_size.unwrap().height {
                    Alignment::End
                } else {
                    Alignment::Start
                };

                let (top, bottom) = match align_y {
                    Alignment::Start => (context_menu_pos.y - 36., 0.),
                    Alignment::End => (0., (window_size.unwrap().height - context_menu_pos.y)),
                    _ => unreachable!(),
                };

                let (left, right) = match align_x {
                    Alignment::Start => (context_menu_pos.x, 0.),
                    Alignment::End => (0., window_size.unwrap().width - context_menu_pos.x),
                    _ => unreachable!(),
                };

                (
                    align_x,
                    align_y,
                    Padding {
                        top,
                        right,
                        bottom,
                        left,
                    },
                )
            } else {
                (Alignment::Center, Alignment::Center, Padding::ZERO)
            }
        };

        let rule1 = || horizontal_rule(1);

        // TODO: filling the button for the entire column is nontrivial
        // maybe future KL will fix it
        // let name = if let Some(texture_name) = texture_name {
        //     println!("texture name is {}", texture_name);
        //     column![text(texture_name), rule1()]
        // } else {
        //     column![]
        // };

        let export = context_menu_button!("export", WaddyMessage::None);
        let copy_to = context_menu_button!("copy_to", WaddyMessage::None);
        let delete = context_menu_button!("delete", WaddyMessage::None);

        // let a = container("abcd").style(style);

        let mut column_display = column![];

        if let Some(texture_name) = texture_name {
            let view_image = context_menu_button!("view", WaddyMessage::None);
            // last_selected is guaranteed to have a value
            let rename = context_menu_button!(
                "Rename",
                WaddyMessage::TileMessage(last_selected.unwrap(), TileMessage::EditEnter)
            );

            column_display = column_display
                .push(context_menu_button!(text(texture_name), WaddyMessage::None))
                .push(rule1())
                .push(view_image)
                .push(rule1())
                .push(rename);
        }

        column_display = column_display
            .push(export)
            .push(rule1())
            .push(copy_to)
            .push(rule1())
            .push(delete);

        container(
            container(
                column_display
                    .align_x(Alignment::Start)
                    .width(Length::Shrink),
            )
            .style(|theme| {
                let palette = theme.palette();
                let extended = theme.extended_palette();

                iced::widget::container::Style {
                    text_color: palette.text.into(),
                    background: iced::Background::Color(palette.background).into(),
                    border: iced::Border {
                        radius: 2.into(),
                        width: 1.,
                        color: extended.background.strong.color,
                    },
                    shadow: iced::Shadow {
                        color: iced::Color::BLACK,
                        offset: iced::Vector::new(8., 8.),
                        blur_radius: 8.,
                    },
                }
            }),
        )
        .padding(padding)
        // fill to make sure that it can align properly
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(align_x)
        .align_y(align_y)
        .into()
    }

    fn update(&mut self, message: ContextMenuMessage) {
        match message {
            ContextMenuMessage::None => {}
            ContextMenuMessage::UpdateLastRightClick(point) => self.last_right_click = point.into(),
            ContextMenuMessage::Toggle(bool) => self.is_enable = bool,
        }
    }
}

struct TextureTile {
    id: iced::widget::text_input::Id,
    name: String,
    dimensions: (u32, u32),
    handle: iced::widget::image::Handle,
    state: TileState,
}

enum TileState {
    None,
    Selected,
    InEdit,
}

impl TileState {
    fn is_selected(&self) -> bool {
        matches!(self, Self::Selected)
    }

    fn is_in_edit(&self) -> bool {
        matches!(self, Self::InEdit)
    }
}

#[derive(Debug, Clone)]
pub enum TileMessage {
    EditRequest,
    EditEnter,
    EditChange(String),
    EditFinish,
    LeftClick,
    RightClick,
    Select,
    Deselect,
}

impl TextureTile {
    fn view(&self) -> Element<TileMessage> {
        let image = iced::widget::image(&self.handle)
            .width(Length::Fixed(TILE_DIMENSION))
            .height(Length::Fixed(TILE_DIMENSION))
            .content_fit(ContentFit::ScaleDown);
        // let image_button = button(image)
        //     .on_press(TileMessage::)
        //     .style(iced::widget::button::text);

        let image_button = iced::widget::mouse_area(image)
            .on_press(TileMessage::LeftClick)
            // tile can sense right click as well
            // this is to make sure that the tile is selected
            .on_right_press(TileMessage::RightClick);

        let dimensions = text(format!("{}x{}", self.dimensions.0, self.dimensions.1));

        // let a =text_input("texture name", self.name.as_str())
        // .on_input(TileMessage::EditChange)
        // .on_submit(TileMessage::EditFinish)
        // // padding zero so it doesn't get bigger
        // .padding(Padding::ZERO)
        // .id(self.id.clone())
        let name = if matches!(self.state, TileState::InEdit) {
            row![text_input("texture name", self.name.as_str())
                .on_input(TileMessage::EditChange)
                .on_submit(TileMessage::EditFinish)
                // padding zero so it doesn't get bigger
                .padding(Padding::ZERO)
                .id(self.id.clone())]
        } else {
            row![mouse_area(self.name.as_str()).on_press(TileMessage::EditRequest)]
        };

        let tile = container(column![image_button, name, dimensions])
            .style(|theme: &iced::Theme| {
                let palette = theme.palette();

                iced::widget::container::Style {
                    text_color: palette.text.into(),
                    background: Some(iced::Background::Color(match self.state {
                        TileState::None => palette.background,
                        TileState::Selected => palette.primary,
                        TileState::InEdit => palette.danger,
                    })),
                    ..Default::default()
                }
            })
            .width(Length::Fixed(TILE_DIMENSION))
            .padding(Padding {
                top: 0.,
                bottom: 0.,
                right: 4.,
                left: 4.,
            });

        tile.into()
    }

    fn update(&mut self, message: TileMessage) {
        match message {
            TileMessage::EditRequest => {}
            TileMessage::EditFinish => self.state = TileState::None,
            TileMessage::LeftClick => {}
            TileMessage::RightClick => {}
            TileMessage::Select => self.state = TileState::Selected,
            TileMessage::Deselect => self.state = TileState::None,
            TileMessage::EditEnter => self.state = TileState::InEdit,
            TileMessage::EditChange(string) => {
                self.name = string;
                self.name.truncate(15);
            }
        }
    }
}

fn get_texture_tiles_from_wad(wad: Wad) -> Vec<TextureTile> {
    wad.entries
        .iter()
        .map(|entry| {
            let wad::types::FileEntry::MipTex(ref image) = entry.file_entry else {
                todo!("extracting image data from non-miptex is yet todo")
            };

            let (pixels, (width, height)) = image.to_rgba();

            let handle = iced::widget::image::Handle::from_rgba(width, height, pixels);
            let name = entry.texture_name();
            let dimensions = entry.file_entry.dimensions();

            TextureTile {
                id: iced::widget::text_input::Id::unique(),
                handle,
                name,
                dimensions,
                state: TileState::None,
            }
        })
        .collect()
}
