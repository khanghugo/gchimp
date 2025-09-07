use std::{
    sync::{Arc, Mutex},
    thread,
};

use gchimp::modules::{loop_wave::loop_wave, split_model::split_model};
use iced::{
    widget::{
        button, checkbox, column, container, horizontal_rule, hover, mouse_area, row, text,
        text_input, tooltip, Button, Column, Text,
    },
    Element,
    Length::{self, Fill},
    Subscription,
};

use crate::{
    gui2::{Program, TabProgram},
    spaced_col, spaced_row, with_tooltip,
};

#[derive(Debug, Clone)]
pub enum MiscMessage {
    None,
    // Split model
    Qc(String),
    QcPicker,
    SplitModelRun,
    // Loop wave
    Wav(String),
    WavPicker,
    WaveLoopRun,
    WaveLoopShouldLoop(bool),
    Status(String),
}

pub struct MiscProgram {
    qc: String,
    wav: String,
    loop_wave_should_loop: bool,
    status: Arc<Mutex<String>>,
}

impl Default for MiscProgram {
    fn default() -> Self {
        Self {
            qc: Default::default(),
            wav: Default::default(),
            status: Arc::new(Mutex::new(String::from("Idle"))),
            loop_wave_should_loop: true,
        }
    }
}

impl TabProgram for MiscProgram {
    fn title(&self) -> &'static str {
        "Misc"
    }

    fn program(&self) -> Program {
        Program::Misc
    }
}

impl MiscProgram {
    pub fn view(&self) -> Element<MiscMessage> {
        let binding = self.status.lock().unwrap();
        let status_text = binding.as_str().to_string();
        let status_text = container(text(status_text)).padding(10);

        column![
            self.split_model_view(),
            horizontal_rule(1),
            self.loop_wave_view(),
            horizontal_rule(1),
            status_text,
        ]
        .into()
    }

    // TODO, focus text on right side
    pub fn update(&mut self, message: MiscMessage) {
        match message {
            MiscMessage::Qc(s) => self.qc = s,
            MiscMessage::SplitModelRun => self.split_model_run(),
            MiscMessage::Status(s) => *self.status.lock().unwrap() = s,
            MiscMessage::QcPicker => {
                if let Some(qc) = rfd::FileDialog::new().add_filter("QC", &["qc"]).pick_file() {
                    let file_name = qc.display().to_string();
                    if file_name.ends_with(".qc") {
                        self.qc = file_name;
                    }
                };
            }
            MiscMessage::Wav(s) => self.wav = s,
            MiscMessage::WavPicker => {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("WAV", &["wav"])
                    .pick_file()
                {
                    let file_name = file.display().to_string();
                    if file_name.ends_with(".wav") {
                        self.wav = file_name;
                    }
                };
            }
            MiscMessage::WaveLoopRun => self.loop_wave_run(),
            MiscMessage::WaveLoopShouldLoop(b) => self.loop_wave_should_loop = b,
            MiscMessage::None => {}
        }
    }

    pub fn subscription(&self) -> Subscription<MiscMessage> {
        Subscription::none()
    }

    fn split_model_view(&self) -> Element<MiscMessage> {
        let title = title("Split Model");
        let file_input = text_input("Choose .qc file", &self.qc)
            .on_input(MiscMessage::Qc)
            .width(Length::Fill);
        let add_button = button("Add").on_press(MiscMessage::QcPicker);
        let run_button = button("Run").on_press(MiscMessage::SplitModelRun);

        let line1 = spaced_row!(row![title, file_input, add_button]);
        let line2 = spaced_row!(row![run_button]);

        spaced_col!(column![line1, line2]).into()
    }

    fn split_model_run(&mut self) {
        self.update(MiscMessage::Status("Running Split Model".to_string()));

        if let Err(err) = split_model(self.qc.as_str()) {
            self.update(MiscMessage::Status(err.to_string()));
        } else {
            self.update(MiscMessage::Status("Done".to_string()));
        }
    }

    fn loop_wave_view(&self) -> Element<MiscMessage> {
        let title = title("Loop Wave");
        let file_input = text_input("Choose .wav file", &self.wav)
            .on_input(MiscMessage::Wav)
            .width(Fill);
        let add_button = button("Add").on_press(MiscMessage::WavPicker);
        let loop_check = with_tooltip!(
            checkbox("Loop", self.loop_wave_should_loop).on_toggle(MiscMessage::WaveLoopShouldLoop),
            "Toggle off to just re-encode audio"
        );
        // let tool_tip = tooltip(
        //     loop_check,
        //     container(text("this is help text")).style(|theme: &iced::Theme| {
        //         let palette = theme.palette();

        //         iced::widget::container::Style {
        //             background: Some(iced::Background::Color(palette.background)),
        //             ..Default::default()
        //         }
        //     }),
        //     tooltip::Position::FollowCursor,
        // );

        let run_button = button("Run").on_press(MiscMessage::WaveLoopRun);

        let line1 = spaced_row!(row![title, file_input, add_button]);
        let line2 = spaced_row!(row![loop_check]);
        let line3 = spaced_row!(row![run_button]);

        spaced_col!(column![line1, line2, line3].spacing(10)).into()
    }

    fn loop_wave_run(&mut self) {
        self.update(MiscMessage::Status("Running Loop Wave".to_string()));

        if let Err(err) = loop_wave(&self.wav, self.loop_wave_should_loop, true) {
            self.update(MiscMessage::Status(err.to_string()));
        } else {
            self.update(MiscMessage::Status("Done".to_string()));
        }
    }
}

fn title(s: &str) -> Element<MiscMessage> {
    text(s).width(80).into()
}
