use std::{fs::OpenOptions, io::Read};

use iced::{widget::button, Element, Task};

#[derive(Debug, Clone)]
pub struct PlayGround;

#[derive(Debug, Clone)]
pub enum PlayGroundMessage {
    ReadRequest,
    ReadDone,
    ReadDone2,
}

impl PlayGround {
    pub fn view(&self) -> Element<PlayGroundMessage> {
        button("click it")
            .on_press(PlayGroundMessage::ReadRequest)
            .into()
    }

    pub fn update(&mut self, message: PlayGroundMessage) -> Task<PlayGroundMessage> {
        match message {
            PlayGroundMessage::ReadRequest => {
                println!("button is pressed");

                Task::perform(
                    async move {
                        let mut file = OpenOptions::new()
                            .read(true)
                            .open("/tmp/aaaa/atnarostar.wad")
                            .unwrap();
                        let mut buf = vec![];

                        file.read_to_end(&mut buf).unwrap();

                        println!("done reading in async move");

                        buf
                    },
                    |res| {
                        println!("Done reading byte in Task");
                        PlayGroundMessage::ReadDone
                    },
                )
            }
            PlayGroundMessage::ReadDone => {
                println!("Updated with ReadDone");

                Task::done(PlayGroundMessage::ReadDone2)
            }
            PlayGroundMessage::ReadDone2 => {
                println!("Updated with ReadDone2");

                Task::none()
            }
        }
    }
}
