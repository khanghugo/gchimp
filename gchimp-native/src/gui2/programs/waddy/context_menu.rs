use iced::{
    widget::{column, container, horizontal_rule, text},
    Alignment, Element, Length, Padding,
};

use crate::{
    context_menu_button,
    gui2::programs::waddy::{TileMessage, WaddyMessage},
};

pub struct ContextMenu {
    pub is_enable: bool,
    // last right click position
    pub last_right_click: Option<iced::Point>,
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self {
            is_enable: false,
            last_right_click: None,
        }
    }
}

pub enum ContextMenuMessage {
    None,
    Toggle(bool),
    UpdateLastRightClick(iced::Point<f32>),
}

impl ContextMenu {
    pub fn view(
        &'_ self,
        last_cursor_pos: Option<iced::Point>,
        window_size: Option<iced::Size<f32>>,
        texture_name: Option<String>,
        last_selected: Option<usize>,
    ) -> Element<'_, WaddyMessage> {
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

    pub fn update(&mut self, message: ContextMenuMessage) {
        match message {
            ContextMenuMessage::None => {}
            ContextMenuMessage::UpdateLastRightClick(point) => self.last_right_click = point.into(),
            ContextMenuMessage::Toggle(bool) => self.is_enable = bool,
        }
    }
}
