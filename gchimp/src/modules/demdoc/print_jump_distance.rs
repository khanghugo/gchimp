use std::str::from_utf8;

use clap::builder::FalseyValueParser;
use dem::{
    hldemo::{ClientDataData, ConsoleCommandData, Demo, FrameData},
    open_demo, parse_netmsg,
    types::ClientDataWeaponData,
    Aux,
};
use glam::{Vec3, Vec3Swizzles};

const EPSILON: f32 = 0.001;

pub fn print_jump_distance(demo: &Demo) {
    let aux = Aux::new();
    let mut prev_origin: Option<&[f32; 3]> = None;
    let mut jump_start: Option<&[f32; 3]> = None;
    let mut already_jumped = false;

    for entry in &demo.directory.entries {
        for (_frame_idx, frame) in entry.frames.iter().enumerate() {
            match &frame.data {
                FrameData::NetMsg((_, netmsg)) => {
                    let (_, v) = parse_netmsg(netmsg.msg, &aux).unwrap();

                    // println!("{:?}", v);
                }
                FrameData::ClientData(ClientDataData { origin, .. }) => {
                    if let Some(start) = jump_start {
                        let end_z = start[2] - 18.;

                        if origin[2] <= end_z + EPSILON && origin[2] >= end_z - EPSILON {
                            let end = origin;
                            let v = Vec3::from_slice(end) - Vec3::from_slice(start);
                            let d = v.xy().length();

                            println!("Real distance is {}", d);
                            println!("Compensated distance is {}", d + 32.);
                            println!("Block distance is {}", v.x.abs().max(v.y.abs()) + 32.);

                            jump_start = None;
                            already_jumped = false;
                        }
                    }

                    if let Some(prev) = prev_origin {
                        if prev[2] < origin[2] && !already_jumped {
                            jump_start = Some(prev);
                            already_jumped = true;
                        } else if prev[2] == origin[2] {
                            // walking up stairs
                            already_jumped = false;
                        }
                    };

                    prev_origin = Some(origin);

                    // println!("{} {}", _frame_idx, origin[2]);
                }
                _ => (),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run() {
        let demo = open_demo("/home/khang/dem/target/LJ WR DEMOS/259_lj_propane.dem").unwrap();
        print_jump_distance(&demo);
    }
}
