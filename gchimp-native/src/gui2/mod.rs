use constants::{DEFAULT_DIMENSIONS, DEFAULT_TEXT_SIZE, PROGRAM_NAME};
use iced::{
    widget::{
        button,
        button::{primary, secondary, success},
        container, horizontal_space, scrollable, text,
    },
    Element, Font, Padding, Pixels, Subscription, Task, Theme,
};
use image::{EncodableLayout, GenericImageView};
use programs::{
    misc::{MiscMessage, MiscProgram},
    playground::{PlayGround, PlayGroundMessage},
    waddy::{WaddyMessage, WaddyProgram},
};

use iced::widget::{column, row};

mod constants;
mod programs;
mod styles;
mod utils;

pub fn gchimp_native_run() -> iced::Result {
    // this is stupid?
    let image_bytes = include_bytes!("../../../media/logo.png");
    let icon_texture = image::load_from_memory(image_bytes).expect("cannot load png icon");
    let (icon_width, icon_height) = icon_texture.dimensions();
    let iced_icon = iced::window::icon::from_rgba(
        icon_texture.to_rgba8().as_bytes().to_owned(),
        icon_width,
        icon_height,
    )
    .expect("cannot convert to iced icon");

    iced::application(PROGRAM_NAME, GchimpNative::update, GchimpNative::view)
        .theme(GchimpNative::theme)
        .settings(iced::Settings {
            id: Some(PROGRAM_NAME.to_string()),
            default_text_size: Pixels(DEFAULT_TEXT_SIZE),
            default_font: Font {
                weight: iced::font::Weight::Medium,
                ..Default::default()
            },
            antialiasing: true,
            ..Default::default()
        })
        .window(iced::window::Settings {
            size: DEFAULT_DIMENSIONS.into(),
            icon: Some(iced_icon),
            ..Default::default()
        })
        .subscription(GchimpNative::subscription)
        .run()
}

trait TabProgram {
    fn title(&self) -> &'static str {
        "my_program"
    }

    fn program(&self) -> Program {
        Program::Misc
    }
}

#[derive(Debug, Clone)]
enum GlobalMessage {
    None,
    Switch(Program),
    MiscMessage(MiscMessage),
    WaddyMessage(WaddyMessage),
    PlayGroundMessage(PlayGroundMessage),
    CycleTheme,
}

struct GchimpNative {
    theme: Theme,
    active: Program,
    misc: MiscProgram,
    waddy: WaddyProgram,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Program {
    Waddy,
    Misc,
    PlayGround,
}

impl Default for GchimpNative {
    fn default() -> Self {
        Self {
            theme: Theme::Oxocarbon,
            misc: Default::default(),
            active: Program::Misc,
            waddy: Default::default(),
        }
    }
}

impl GchimpNative {
    fn view(&self) -> Element<GlobalMessage> {
        let main_view = match self.active {
            Program::Waddy => self
                .waddy
                .view()
                .map(move |message| GlobalMessage::WaddyMessage(message)),
            Program::Misc => self
                .misc
                .view()
                .map(move |message| GlobalMessage::MiscMessage(message)),
            Program::PlayGround => PlayGround
                .view()
                .map(move |message| GlobalMessage::PlayGroundMessage(message)),
        };

        // let cycle = iced::widget::button("cycle").on_press(GlobalMessage::CycleTheme);

        column![self.tab_menu(), main_view].into()
    }

    fn update(&mut self, message: GlobalMessage) -> Task<GlobalMessage> {
        match message {
            GlobalMessage::None => Task::none(),
            GlobalMessage::CycleTheme => {
                let current_theme_idx = iced::Theme::ALL
                    .iter()
                    .position(|x| *x == self.theme)
                    .unwrap();
                let next = (current_theme_idx + 1) % iced::Theme::ALL.len();
                self.theme = iced::Theme::ALL[next].clone();
                println!("current is {:?}", self.theme);

                Task::none()
            }
            GlobalMessage::Switch(program) => {
                self.active = program;

                // match self.active {
                //     // Get newer window size when switch tab
                //     Program::Waddy => self
                //         .update(GlobalMessage::WaddyMessage(WaddyMessage::GetWindowSize))
                //         .map(|_| GlobalMessage::None),
                //     Program::Misc => Task::none(),
                // }

                Task::none()
            }
            GlobalMessage::MiscMessage(misc_message) => {
                self.misc.update(misc_message);

                Task::none()
            }
            GlobalMessage::WaddyMessage(waddy_message) => self
                .waddy
                .update(waddy_message)
                .map(|res| GlobalMessage::WaddyMessage(res)),
            GlobalMessage::PlayGroundMessage(play_ground_message) => PlayGround
                .update(play_ground_message)
                .map(|res| GlobalMessage::PlayGroundMessage(res)),
        }
    }

    fn subscription(&self) -> Subscription<GlobalMessage> {
        match self.active {
            Program::Waddy => self.waddy.subscription().map(GlobalMessage::WaddyMessage),
            Program::Misc => self.misc.subscription().map(GlobalMessage::MiscMessage),
            Program::PlayGround => Subscription::none(),
        }
        // iced::event::listen().map(|event| match event {
        //     iced::Event::Window(event) => match event {
        //         iced::window::Event::FileDropped(path_buf) => {
        //             println!("dropped file is {}", path_buf.display());
        //             GlobalMessage::None
        //         }
        //         _ => GlobalMessage::None,
        //     },
        //     iced::Event::Mouse(event) => match event {
        //         iced::mouse::Event::CursorMoved { position } => {
        //             // println!("cursor moved");
        //             GlobalMessage::None
        //         }
        //         _ => GlobalMessage::None,
        //     },
        //     _ => GlobalMessage::None,
        // })
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn tab_menu(&self) -> Element<GlobalMessage> {
        let tab_button = |i, program: Program| {
            button(i).on_press(GlobalMessage::Switch(program)).style({
                if self.active == program {
                    iced::widget::button::secondary
                } else {
                    iced::widget::button::text
                }
            })
        };

        let misc = tab_button(self.misc.title(), self.misc.program());
        let waddy = tab_button(self.waddy.title(), self.waddy.program());
        let test = tab_button("test", Program::PlayGround);

        container(
            scrollable(
                row![
                    misc,
                    waddy,
                    test,
                    button("alo"),
                    button("alo"),
                    button("alo"),
                    button("alo"),
                    button("alo"),
                    button("alo"),
                    button("alo"),
                    button("alo"),
                    button("alo"),
                ]
                .spacing(2)
                .padding(Padding {
                    top: 5.,
                    bottom: 5.,
                    ..Default::default()
                }),
            )
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
                    .width(Pixels(1.))
                    .scroller_width(Pixels(1.)),
            )),
        )
        .padding(Padding {
            right: 10.,
            left: 10.,
            ..Default::default()
        })
        .into()
    }
}
