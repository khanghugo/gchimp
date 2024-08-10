use dem::hldemo::{Demo, FrameData};
use dem::types::{EngineMessage, NetMessage, SvcTime};
use dem::{parse_netmsg, write_netmsg, Aux};

use crate::wrap_message;

use self::add_keys::add_keys;
use self::add_speedometer::add_speedometer;

pub mod add_keys;
pub mod add_speedometer;

pub struct KzAddOns {
    keys: bool,
    speedometer: bool,
}

impl Default for KzAddOns {
    fn default() -> Self {
        Self::new()
    }
}

impl KzAddOns {
    pub fn new() -> Self {
        Self {
            keys: false,
            speedometer: false,
        }
    }

    pub fn add_keys(&mut self) -> &mut Self {
        self.keys = true;
        self
    }

    pub fn add_speedometer(&mut self) -> &mut Self {
        self.speedometer = true;
        self
    }
}

#[derive(Debug)]
pub struct KzInfo<'a> {
    // First 3 members could only be found in netmessage.
    // Frame 0 0 is netmessage.
    // Frame 1 0 is not netmessage.
    forward: f32,
    side: f32,
    up: f32,
    origin: [f32; 3],
    _viewangles: [f32; 3],
    // velocity: [f32; 3],
    buttons: u16,
    _movetype: i32,
    // weapon: i32,
    _flags: u32,
    commands: &'a [u8],
    frametime: f32,
}

impl<'a> KzInfo<'a> {
    fn new(origin: [f32; 3], viewangles: [f32; 3], frametime: f32) -> Self {
        Self {
            forward: 0.,
            side: 0.,
            up: 0.,
            origin,
            _viewangles: viewangles,
            // velocity: VEC3EMPTY,
            buttons: 0,
            _movetype: 0,
            // weapon,
            _flags: 0,
            commands: &[],
            // accumulative
            frametime,
        }
    }
}

pub fn add_kz_stats(demo: &mut Demo, builder: impl FnOnce(&mut KzAddOns)) {
    let aux = Aux::new();

    let mut addons = KzAddOns::new();
    builder(&mut addons);

    for entry in demo.directory.entries.iter_mut() {
        let mut curr: Option<KzInfo> = None;
        let mut prev: Option<KzInfo> = None;
        let mut should_push = false;

        // Alternative strat to add to prev frame instead but it doesn't work as well
        {
            // let frame_iter = entry.frames.iter().filter_map(|frame| {
            //     if let FrameData::NetMsg((_, netmsg)) = &frame.data {
            //         Some(netmsg)
            //     } else {
            //         None
            //     }
            // });

            // let mut to_add = frame_iter
            //     .clone()
            //     .zip(frame_iter.skip(1))
            //     .map(|(curr, next)| {
            //         let (_, messages) = parse_netmsg(curr.msg, &aux).unwrap();

            //         for message in &messages {
            //             if let NetMessage::EngineMessage(x) = &message {
            //                 if let EngineMessage::SvcTime(SvcTime { time }) = x.as_ref() {
            //                     info = Some(KzInfo::new(
            //                         curr.info.ref_params.vieworg,
            //                         curr.info.ref_params.viewangles,
            //                         *time,
            //                     ));
            //                 }
            //             }
            //         }

            //         if let Some(ref mut info) = info {
            //             info.forward = curr.info.usercmd.forwardmove;
            //             info.side = curr.info.usercmd.sidemove;
            //             info.up = curr.info.usercmd.upmove;
            //             info.buttons = curr.info.usercmd.buttons;
            //             // movetype?
            //             // weapon?
            //             // flags?
            //         }

            //         let next_time = {
            //             let mut res = 0.;
            //             let (_, messages) = parse_netmsg(next.msg, &aux).unwrap();

            //             for message in &messages {
            //                 if let NetMessage::EngineMessage(x) = &message {
            //                     if let EngineMessage::SvcTime(SvcTime { time }) = x.as_ref() {
            //                         res = *time;
            //                     }
            //                 }
            //             }

            //             res
            //         };

            //         let next_info = Some(KzInfo::new(
            //             next.info.ref_params.vieworg,
            //             next.info.ref_params.viewangles,
            //             next_time,
            //         ));

            //         let mut to_add = vec![];

            //         if addons.speedometer {
            //             if let Some(temp_entity) = add_speedometer(info.as_ref(), next_info.as_ref()) {
            //                 to_add.push(wrap_message!(SvcTempEntity, temp_entity));
            //             }
            //         }

            //         if addons.keys {
            //             if let Some(temp_entity) = add_keys(info.as_ref()) {
            //                 to_add.push(wrap_message!(SvcTempEntity, temp_entity));
            //             }
            //         }

            //         to_add
            //     })
            //     .collect::<Vec<Vec<NetMessage>>>();

            // println!("to add length is {}", to_add.len());

            // entry
            //     .frames
            //     .iter_mut()
            //     .filter_map(|frame| {
            //         if let FrameData::NetMsg((_, netmsg)) = &mut frame.data {
            //             Some(netmsg)
            //         } else {
            //             None
            //         }
            //     })
            //     .zip(to_add.iter_mut())
            //     .for_each(|(netmsg, mut to_add)| {
            //         let (_, mut messages) = parse_netmsg(netmsg.msg, &aux).unwrap();

            //         messages.append(&mut to_add);

            //         let write = write_netmsg(messages, &aux);
            //         netmsg.msg = write.leak();
            //     });
        }

        for frame in &mut entry.frames {
            match &mut frame.data {
                FrameData::NetMsg((_, netmsg)) => {
                    let (_, mut messages) = parse_netmsg(netmsg.msg, &aux).unwrap();

                    for message in &messages {
                        if let NetMessage::EngineMessage(x) = &message {
                            if let EngineMessage::SvcTime(SvcTime { time }) = x.as_ref() {
                                prev = curr;
                                curr = Some(KzInfo::new(
                                    netmsg.info.ref_params.vieworg,
                                    netmsg.info.ref_params.viewangles,
                                    *time,
                                ));

                                should_push = true;
                            }
                        }
                    }

                    if let Some(ref mut curr) = curr {
                        curr.forward = netmsg.info.usercmd.forwardmove;
                        curr.side = netmsg.info.usercmd.sidemove;
                        curr.up = netmsg.info.usercmd.upmove;
                        curr.buttons = netmsg.info.usercmd.buttons;
                        // movetype?
                        // weapon?
                        // flags?
                    }

                    if should_push {
                        if addons.speedometer {
                            if let Some(temp_entity) = add_speedometer(prev.as_ref(), curr.as_ref())
                            {
                                messages.push(wrap_message!(SvcTempEntity, temp_entity));
                            }
                        }

                        if addons.keys {
                            if let Some(temp_entity) = add_keys(curr.as_ref()) {
                                messages.push(wrap_message!(SvcTempEntity, temp_entity));
                            }
                        }
                        should_push = false;
                    }

                    let write = write_netmsg(messages, &aux);
                    netmsg.msg = write.leak();
                }
                // FrameData::ClientData(client_data) => {
                //     // prev = curr;
                //     // curr = Some(KzInfo::new(client_data.origin, client_data.viewangles));
                // }
                FrameData::ConsoleCommand(command) => {
                    if let Some(ref mut curr) = curr {
                        curr.commands = command.command;
                    }
                }
                _ => (),
            }
        }
    }
}

trait CoordConversion {
    fn coord_conversion(&self) -> i16;
}

impl CoordConversion for f32 {
    fn coord_conversion(&self) -> i16 {
        (self * 8192.).round() as i16
    }
}
