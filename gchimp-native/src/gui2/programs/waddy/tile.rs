use iced::{
    widget::{column, container, mouse_area, row, text, text_input},
    ContentFit, Element, Length, Padding,
};
use wad::types::Wad;

const TILE_DIMENSION: f32 = 134.;

pub struct WaddyTile {
    pub id: iced::widget::text_input::Id,
    pub label: String,
    pub dimensions: (u32, u32),
    pub handle: iced::widget::image::Handle,
}

#[derive(Debug, Clone)]
pub enum TileMessage {
    EditRequest,
    EditEnter,
    EditChange(String),
    EditFinish,
    EditCancel,
    LeftClick,
    RightClick,
}

impl WaddyTile {
    pub fn view(&'_ self, selected: bool, in_edit: bool) -> Element<'_, TileMessage> {
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
        let name = if in_edit {
            row![text_input("texture name", self.label.as_str())
                .on_input(TileMessage::EditChange)
                .on_submit(TileMessage::EditFinish)
                // padding zero so it doesn't get bigger
                .padding(Padding::ZERO)
                .id(self.id.clone())]
        } else {
            row![mouse_area(self.label.as_str())]
        };

        let tile = container(column![image_button, name, dimensions])
            .style(move |theme: &iced::Theme| {
                let palette = theme.palette();

                iced::widget::container::Style {
                    text_color: palette.text.into(),
                    background: Some(iced::Background::Color(if selected {
                        palette.primary
                    } else if in_edit {
                        palette.danger
                    } else {
                        palette.background
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

    // a tile can only update intrinsic data
    pub fn update(&mut self, message: TileMessage) {
        match message {
            TileMessage::EditChange(string) => {
                self.label = string;
                self.label.truncate(15);
            }
            TileMessage::EditFinish
            | TileMessage::EditCancel
            | TileMessage::EditRequest
            | TileMessage::EditEnter => {}
            // clicks should only change grid state, not tile state
            TileMessage::LeftClick | TileMessage::RightClick => {}
        }
    }
}

pub fn get_texture_tiles_from_wad(wad: Wad) -> Vec<WaddyTile> {
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

            WaddyTile {
                id: iced::widget::text_input::Id::unique(),
                handle,
                label: name,
                dimensions,
            }
        })
        .collect()
}
