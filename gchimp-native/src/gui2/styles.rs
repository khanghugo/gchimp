use iced::{
    alignment,
    widget::{button, row, text},
    Length,
};
use iced_aw::iced_fonts::required::{icon_to_string, RequiredIcons};

#[macro_export]
macro_rules! spaced_row {
    ($row:expr) => {{
        use iced::Alignment::Center;
        use iced::Padding;

        $row.align_y(Center)
            .padding(Padding {
                left: 4.,
                right: 4.,
                ..Default::default()
            })
            .spacing(4)
    }};
}

#[macro_export]
macro_rules! spaced_col {
    ($row:expr) => {{
        use iced::Padding;

        $row.padding(Padding {
            top: 4.,
            bottom: 4.,
            ..Default::default()
        })
        .spacing(4)
    }};
}

#[macro_export]
macro_rules! with_tooltip {
    ($element:expr, $tip:literal) => {{
        use iced::widget::container;
        use iced::widget::tooltip;

        tooltip(
            $element,
            container(text($tip)).style(|theme: &iced::Theme| {
                let palette = theme.palette();

                iced::widget::container::Style {
                    background: Some(iced::Background::Color(palette.background)),
                    ..Default::default()
                }
            }),
            tooltip::Position::FollowCursor,
        )
    }};
}

#[macro_export]
macro_rules! context_menu_button {
    ($content:expr, $map:expr) => {{
        use iced::widget::button::Status;
        use iced::widget::button::Style;

        iced::widget::mouse_area(
            iced::widget::button($content)
                .style(|theme: &iced::Theme, status| {
                    let palette = theme.extended_palette();

                    let base = Style {
                        text_color: palette.background.base.text,
                        ..Style::default()
                    };

                    match status {
                        Status::Active | Status::Pressed => base,
                        Status::Hovered => Style {
                            background: Some(iced::Background::Color(
                                palette.secondary.strong.color,
                            )),
                            ..base
                        },
                        Status::Disabled => Style {
                            background: base
                                .background
                                .map(|background| background.scale_alpha(0.5)),
                            text_color: base.text_color.scale_alpha(0.5),
                            ..base
                        },
                    }
                })
                .on_press($map),
        )
        .on_right_press($map)
    }};
}

// fn base_button<'a, T>(content: impl Into<T<'a, T>>, msg: T) -> button::Button<'a, T>
// where
//     T: std::fmt::Debug + Clone,
// {
//     button(content)
//         .padding([4, 8])
//         .style(iced::widget::button::primary)
//         .on_press(msg)
// }

#[macro_export]
macro_rules! menu_base_button {
    ($content:expr, $msg:expr) => {{
        iced::widget::button($content)
            .padding([4, 8])
            .style(iced::widget::button::primary)
            .on_press($msg)
    }};
}

#[macro_export]
macro_rules! menu_submenu_button {
    ($label:literal, $msg:expr) => {{
        use crate::menu_base_button;
        use iced::alignment;
        use iced::Alignment;
        use iced::Length;
        use iced_aw::iced_fonts::required::{icon_to_string, RequiredIcons};

        menu_base_button!(
            iced::widget::row![
                iced::widget::text($label)
                    .width(Length::Fill)
                    .align_y(alignment::Vertical::Center),
                iced::widget::text(icon_to_string(RequiredIcons::CaretRightFill))
                    // .font(REQUIRED_FONT)
                    .width(Length::Shrink)
                    .align_y(alignment::Vertical::Center),
            ]
            .align_y(Alignment::Center),
            $msg
        )
        .width(Length::Fill)
    }};
}

#[macro_export]
macro_rules! menu_labeled_button {
    ($label:literal, $msg:expr) => {{
        use crate::menu_base_button;
        use iced::widget::text;

        menu_base_button!(
            text($label).align_y(iced::alignment::Vertical::Center),
            $msg
        )
        .width(iced::Length::Fill)
    }};
}
