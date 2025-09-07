use iced::{
    border::Radius,
    widget::{column, container, mouse_area, row, text, text_input},
    Border, ContentFit, Element, Length, Padding,
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

        let image_button = iced::widget::mouse_area(image)
            .on_press(TileMessage::LeftClick)
            // tile can sense right click as well
            // this is to make sure that the tile is selected
            .on_right_press(TileMessage::RightClick);

        let dimensions = text(format!("{}x{}", self.dimensions.0, self.dimensions.1));

        let name = if in_edit {
            row![text_input("texture name", self.label.as_str())
                .on_input(TileMessage::EditChange)
                .on_submit(TileMessage::EditFinish)
                // padding zero so it doesn't get bigger
                .padding(Padding::ZERO)
                .id(self.id.clone())]
        } else {
            let label_clickable =
                mouse_area(self.label.as_str()).on_press(TileMessage::EditRequest);

            row![label_clickable]
        };

        let tile = container(column![image_button, name, dimensions])
            .style(move |theme: &iced::Theme| {
                let palette = theme.palette();

                iced::widget::container::Style {
                    text_color: palette.text.into(),
                    background: Some(iced::Background::Color(
                        // branch in_edit first
                        if in_edit {
                            palette.danger
                        } else if selected {
                            palette.primary
                        } else {
                            palette.background
                        },
                    )),
                    border: Border {
                        color: iced::Color::WHITE.scale_alpha(0.1),
                        width: 0.5,
                        radius: Radius::new(1.).bottom(4.),
                    },
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

    // no update function
    // pub fn update(&mut self, message: TileMessage) {}
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
