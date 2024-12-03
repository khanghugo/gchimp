use std::cell::RefCell;
use std::path::Path;

use bsp::Bsp;

use dem::hldemo::{
    ClientDataData, Demo, DemoBufferData, Directory, DirectoryEntry, Frame, FrameData, Header,
    NetMsgData, NetMsgFrameType,
};
use dem::parse_netmsg;
use dem::types::{
    Delta, EntityS, EntityState, EntityStateDelta, OriginCoord, Resource, SvcDeltaPacketEntities,
    SvcNewMovevars, SvcPacketEntities, SvcResourceList, SvcServerInfo, SvcSetView, SvcSignOnNum,
    SvcSound, SvcSpawnBaseline,
};
use dem::{
    bitvec::{bitvec, order::Lsb0},
    nbit_num, nbit_str,
    netmsg_doer::Doer,
    Aux,
};
use nom::{number::complete::float, sequence::tuple};

use crate::utils::dem_stuffs::get_ghost::get_ghost;
use crate::{
    get_cs_delta_msg, insert_packet_entity_state_delta_with_index,
    insert_packet_entity_state_with_index, modules::demdoc::ResourceType, rand_int_range,
};

use super::{Buttons, NetMsgDataMethods};

const DEMO_BUFFER_SIZE: [u8; 8] = [1, 0, 0, 0, 0, 0, 180, 66];
const DEFAULT_IN_SEQ: i32 = 143791;
const STEP_TIME: f32 = 0.3;

const MAX_PLAYERS: i32 = 1;

// Eh, maybe someone can spot this and use for different mod.
const GAME_DIR: &str = "cstrike";

pub fn ghost_to_demo<'a>(ghost_file_name: &'a Path, map_file_name: &'a Path) -> Demo<'a> {
    // need to mutate the aux data or we won't be able to write anything with delta
    let aux = Aux::new();

    let mut map_name = vec![0u8; 260];
    let map_file_name_stem = map_file_name.file_stem().unwrap().to_str().unwrap();
    map_name[..map_file_name_stem.len()].copy_from_slice(map_file_name_stem.as_bytes());

    let mut game_dir = vec![0u8; 260];
    game_dir[..GAME_DIR.len()].copy_from_slice(GAME_DIR.as_bytes());

    let header = Header {
        demo_protocol: 5,
        net_protocol: 48,
        map_name: map_name.leak(),
        game_dir: game_dir.leak(),
        map_crc: 0,          // doesnt matter
        directory_offset: 1, // will be corrected when written
    };

    let mut entry0_desc = vec![0u8; 64];
    let entry0_name = "LOADING";
    entry0_desc[..entry0_name.len()].copy_from_slice(entry0_name.as_bytes());

    let entry0 = DirectoryEntry {
        entry_type: 0, // 0 for LOADING
        description: entry0_desc.leak(),
        flags: 0,
        cd_track: -1,
        track_time: 0.0, // doesnt matter
        frame_count: 0,
        offset: 0,      // will be corrected when written
        file_length: 1, // doesnt matter
        frames: vec![],
    };

    let mut entry1_desc = vec![0u8; 64];
    let entry1_name = "Normal";
    entry1_desc[..entry1_name.len()].copy_from_slice(entry1_name.as_bytes());

    let entry1 = DirectoryEntry {
        entry_type: 1, // 1 for Normal
        description: entry1_desc.leak(),
        flags: 0,
        cd_track: -1,
        track_time: 0.0, // doesnt matter
        frame_count: 0,
        offset: 0,      // will be corrected when written
        file_length: 1, // doesnt matter
        frames: vec![],
    };

    let directory = Directory {
        entries: vec![entry0, entry1],
    };

    let mut demo = Demo { header, directory };

    // final steps
    let (game_resource_index_start, packet_entities, delta_packet_entities) =
        insert_base_netmsg(&mut demo, map_file_name, &aux);
    insert_ghost(
        &mut demo,
        ghost_file_name.to_str().unwrap(),
        None,
        None,
        game_resource_index_start,
        packet_entities,
        delta_packet_entities,
        &aux,
    );

    demo
}

#[derive(Debug)]
struct BaselineEntity<'a> {
    index: usize,
    properties: std::collections::HashMap<&'a str, &'a str>,
    modelindex: usize,
    delta: Delta,
}

// array of allowed render objects in demo
const BASELINE_ENTITIES_BRUSH: &[&str] = &["func_door", "func_illusionary"];
const BASELINE_ENTITIES_CYCLER: &[&str] = &["cycler_sprite", "cycler"];

use nom::character::complete::space0;
use nom::combinator::map;
use nom::IResult;
fn parse_3_f32(i: &str) -> IResult<&str, (f32, f32, f32)> {
    map(
        tuple((float, space0, float, space0, float)),
        |(i1, _, i2, _, i3)| (i1, i2, i3),
    )(i)
}

/// frame 0: SvcServerInfo SvcDeltaDescription SvcSetView SvcNewMovevars
///
/// SvcSetView needs setting to 1 otherwise game crash
///
/// SvcNewMovevars is needed otherwise game black screen
///
/// SvcServerInfo: wrong checksum ok
///
/// frame 1: SvcResourceList
///
/// frame 2: 8 nopes, omittable
///
/// frame 3: SvcSpawnBaseline, SvcSignOnNum(_) = 1,
///
/// frame 4: svcpackent entity
///
/// returns the index of game resources for ghost to generate footstep
fn insert_base_netmsg(
    demo: &mut Demo,
    map_file_name: &Path,
    aux: &RefCell<Aux>,
) -> (usize, Vec<u8>, SvcDeltaPacketEntities) {
    // add maps entities first with its models, named "*{number}" and so on until we are done
    // by then we can insert our own custom files
    // bsp is still cached first as 0
    // each baseline_entities will have `model` key. To insert that into baseline, we have to
    // translate that into `modelindex` instead.
    let bsp_file = Bsp::from_file(map_file_name).unwrap();
    let bsp_entities = bsp_file.entities;
    // println!("{:?}", bsp_entities);

    let baseline_entities: Vec<BaselineEntity<'_>> = bsp_entities
        .iter()
        .enumerate()
        // .skip(33) // skip 33 because 0 is bsp and 1-32 are players
        .filter(|(_, ent)| {
            ent.get("classname")
                .map(|classname| {
                    [BASELINE_ENTITIES_BRUSH, BASELINE_ENTITIES_CYCLER]
                        .concat()
                        .contains(&classname.as_str())
                })
                .is_some_and(|x| x)
        })
        // after this filtering, we know which order of resourcelist will go,
        // so we can just enumerate them again and we just need to go with that order
        .enumerate()
        .map(|(modelindex, (index, ent))| {
            let mut delta = Delta::new();

            // from ent.properties, we don't have null terminator
            // but inserting into our delta we need null terminator
            if let Some(property) = ent.get("rendermode") {
                delta.insert(
                    "rendermode\0".to_owned(),
                    property.parse::<i32>().unwrap_or(0).to_le_bytes().to_vec(),
                );
            }

            if let Some(property) = ent.get("renderamt") {
                delta.insert(
                    "renderamt\0".to_owned(),
                    property.parse::<i32>().unwrap_or(0).to_le_bytes().to_vec(),
                );
            }

            if let Some(property) = ent.get("origin") {
                let (_, (x, y, z)) = parse_3_f32(property).unwrap();

                delta.insert("origin[0]\0".to_owned(), x.to_le_bytes().to_vec());
                delta.insert("origin[1]\0".to_owned(), y.to_le_bytes().to_vec());
                delta.insert("origin[2]\0".to_owned(), z.to_le_bytes().to_vec());
            }

            if let Some(property) = ent.get("angles") {
                let (_, (x, y, z)) = parse_3_f32(property).unwrap();

                delta.insert("angles[0]\0".to_owned(), x.to_le_bytes().to_vec());
                delta.insert("angles[1]\0".to_owned(), y.to_le_bytes().to_vec());
                delta.insert("angles[2]\0".to_owned(), z.to_le_bytes().to_vec());
            }

            delta.insert(
                "modelindex\0".to_owned(),
                ((modelindex + 2) as i32).to_le_bytes().to_vec(),
            );

            BaselineEntity {
                index: index + MAX_PLAYERS as usize, // at this point the index is nicely offset by 1 just fine
                properties: ent.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect(),
                modelindex: modelindex + 2, // 1 is bsp, 0 is unused.
                delta,
            }
        })
        .collect();

    let game_dir = format!("{}\0", GAME_DIR);
    let map_file_name = format!(
        "maps/{}\0",
        map_file_name.file_name().unwrap().to_str().unwrap()
    );

    let server_info = SvcServerInfo {
        protocol: 48,
        spawn_count: 5, // ?
        map_checksum: 0,
        client_dll_hash: vec![0u8; 16],
        max_players: MAX_PLAYERS as u8,
        player_index: 0,
        is_deathmatch: 0,
        game_dir: game_dir.as_bytes().to_vec(),
        hostname: b"Ghost Demo Replay\0".to_vec(),
        map_file_name: map_file_name.as_bytes().to_vec(),
        map_cycle: b"a\0".to_vec(), // must be null string
        unknown: 0u8,
    };
    let server_info = server_info.write(aux);

    let dds: Vec<u8> = get_cs_delta_msg!()
        .iter()
        .flat_map(|dd| dd.write(aux))
        .collect();

    // parse delta again just so that we mutate our Aux
    parse_netmsg(dds.as_slice(), aux).unwrap();

    let set_view = SvcSetView { entity_index: 1 }; // always 1
    let set_view = set_view.write(aux);

    let new_movevars = SvcNewMovevars {
        gravity: 800.,
        stop_speed: 75.,
        max_speed: 320.,
        spectator_max_speed: 500.,
        accelerate: 5.,
        airaccelerate: 10.,
        water_accelerate: 10.,
        friction: 4.,
        edge_friction: 2.,
        water_friction: 1.,
        ent_garvity: 1.,
        bounce: 1.,
        step_size: 18.,
        max_velocity: 2000.,
        z_max: 409600.,
        wave_height: 0.,
        footsteps: 1,
        roll_angle: 0.,
        roll_speed: -1.9721523e-31, // have to use these magic numbers to work
        sky_color: vec![-1.972168e-31, -1.972168e-31, 9.4e-44],
        sky_vec: vec![-0.0, 2.68e-43, 2.7721908e20],
        sky_name: [0].to_vec(),
    };
    let new_movevars = new_movevars.write(aux);

    // bsp is always 1, then func_door and illusionary and whatever renders
    // maps resources first
    let bsp = Resource {
        type_: nbit_num!(ResourceType::Model, 4),
        name: nbit_str!(map_file_name),
        index: nbit_num!(1, 12),
        size: nbit_num!(0, 3 * 8),
        flags: nbit_num!(1, 3),
        md5_hash: None,
        has_extra_info: false,
        extra_info: None,
    };

    let bsp_entities_resource: Vec<Resource> = baseline_entities
        .iter()
        .map(|ent| {
            Resource {
                type_: nbit_num!(ResourceType::Model, 4), // blocks and .mdl are all type 2
                name: nbit_str!(format!("{}\0", ent.properties.get("model").unwrap())),
                index: nbit_num!(ent.modelindex, 12), // this is modelindex
                size: nbit_num!(0, 3 * 8),
                flags: nbit_num!(1, 3), // this could be interpolation flag?
                md5_hash: None,
                has_extra_info: false,
                extra_info: None,
            }
        })
        .collect();

    // after that, we can have our own models, game resources later
    let game_resource_index_start = baseline_entities.len() + 2;

    let v_usp = Resource {
        type_: nbit_num!(ResourceType::Skin, 4),
        name: nbit_str!("models/v_usp.mdl\0"),
        index: nbit_num!(game_resource_index_start, 12),
        size: nbit_num!(0, 3 * 8),
        flags: nbit_num!(0, 3),
        md5_hash: None,
        has_extra_info: false,
        extra_info: None,
    };

    let pl_steps: Vec<Resource> = (1..=4) // range like this is awkward
        .map(|i| Resource {
            type_: nbit_num!(ResourceType::Sound, 4),
            name: nbit_str!(format!("player/pl_step{}.wav\0", i)),
            index: nbit_num!(game_resource_index_start + i, 12), // remember to increment
            size: nbit_num!(0, 3 * 8),
            // TODO not sure what the flag does
            flags: nbit_num!(0, 3),
            md5_hash: None,
            has_extra_info: false,
            extra_info: None,
        })
        .collect();

    // add resources here
    // the order doesn't matter because we already specify the resource index
    let resources = [vec![bsp, v_usp], pl_steps, bsp_entities_resource].concat();

    let resource_list = SvcResourceList {
        resource_count: nbit_num!(resources.len(), 12),
        resources,
        consistencies: vec![],
    };
    let resource_list = resource_list.write(aux);

    let worldspawn = EntityS {
        entity_index: 0, // worldspawn is index 0
        index: nbit_num!(0, 11),
        type_: nbit_num!(1, 2),
        delta: Delta::from([
            ("movetype\0".to_owned(), vec![7, 0, 0, 0]),
            ("modelindex\0".to_owned(), vec![1, 0, 0, 0]), // but modelindex is 1
            ("solid\0".to_owned(), vec![4, 0]),
        ]),
    };

    let bsp_entities_baseline: Vec<EntityS> = baseline_entities
        .iter()
        .map(|ent| EntityS {
            entity_index: ent.index as u16,
            index: nbit_num!(ent.index, 11),
            type_: nbit_num!(1, 2),
            delta: ent.delta.to_owned(),
        })
        .collect();

    let spawn_baseline_entities = [vec![worldspawn], bsp_entities_baseline].concat();

    // max_client should be 1 because we are playing demo and it is OK.
    let spawn_baseline = SvcSpawnBaseline {
        entities: spawn_baseline_entities,
        total_extra_data: nbit_num!(0, 6),
        extra_data: vec![],
    };
    let spawn_baseline = spawn_baseline.write(aux);

    let sign_on_num = SvcSignOnNum { sign: 1 };
    let sign_on_num = sign_on_num.write(aux);

    // making entities appearing
    // packet entities is not enough
    // we need delta packet entities also to make the entities appear
    // svcpacketentity is almost redundant but just add it there to make sure
    let player_entity_state = EntityState {
        entity_index: 1,
        increment_entity_number: true,
        is_absolute_entity_index: false.into(),
        absolute_entity_index: None,
        entity_index_difference: None,
        has_custom_delta: false,
        has_baseline_index: false,
        baseline_index: None,
        delta: Delta::new(),
    };

    let mut entity_states = vec![player_entity_state];

    // macro aboose
    baseline_entities.iter().for_each(|ent| {
        // println!("{}", ent.index);
        insert_packet_entity_state_with_index!(entity_states, Delta::new(), ent.index as u16);
    });

    // println!("{:?}", entity_states);

    let packet_entities = SvcPacketEntities {
        entity_count: nbit_num!(entity_states.len(), 16), // has to match the length, of EntityState
        entity_states,
    };
    let packet_entities = packet_entities.write(aux);

    let player_entity_state_delta = EntityStateDelta {
        entity_index: 1,
        remove_entity: false,
        is_absolute_entity_index: false,
        absolute_entity_index: None,
        entity_index_difference: nbit_num!(1, 6).into(),
        has_custom_delta: false.into(),
        delta: Delta::new().into(),
    };

    let mut entity_states_delta: Vec<EntityStateDelta> = vec![player_entity_state_delta];
    // let mut entity_states_delta: Vec<EntityStateDelta> = vec![];

    // let mut count = 50;
    baseline_entities.iter().for_each(|ent| {
        // if count > 0 {
        insert_packet_entity_state_delta_with_index!(
            entity_states_delta,
            ent.delta.to_owned(),
            // Delta::new(),
            ent.index as u16
        );

        // count = count - 1;
        // }
    });

    let delta_packet_entities = SvcDeltaPacketEntities {
        entity_count: nbit_num!(entity_states_delta.len(), 16),
        delta_sequence: nbit_num!((DEFAULT_IN_SEQ & 0xff) - 1, 8), // otherwise entity flush happens
        entity_states: entity_states_delta,
    };
    // let delta_packet_entities_byte =
    // DeltaPacketEntities::write(delta_packet_entities, &mut get_cs_delta_decoder_table!(), 1);

    // println!("{}", baseline_entities.len());

    let mut new_netmsg_data = NetMsgData::new(2);
    new_netmsg_data.msg = [
        server_info,
        dds,
        set_view,
        new_movevars,
        resource_list,
        spawn_baseline,
        sign_on_num,
        packet_entities.to_owned(),
        // delta_packet_entities,
    ]
    .concat()
    .leak();

    let netmsg_framedata = FrameData::NetMsg((NetMsgFrameType::Start, new_netmsg_data));
    let netmsg_frame = Frame {
        time: 0.,
        frame: 0,
        data: netmsg_framedata,
    };

    demo.directory.entries[0].frames.push(netmsg_frame);
    demo.directory.entries[0].frame_count += 1;

    (
        game_resource_index_start,
        packet_entities,
        delta_packet_entities,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn insert_ghost(
    demo: &mut Demo,
    ghost_file_name: &str,
    override_frametime: Option<f32>,
    override_fov: Option<f32>,
    game_resource_index_start: usize,
    packet_entities: Vec<u8>,
    mut delta_packet_entities: SvcDeltaPacketEntities,
    aux: &RefCell<Aux>,
) {
    // setup
    let ghost_info = get_ghost(ghost_file_name).unwrap();

    // set directory entry info
    let entry1 = &mut demo.directory.entries[1];

    // some tracking stuffs
    let mut time = 0.;
    let mut time_step = STEP_TIME;
    let mut last_pos: [f32; 3] = [0.; 3];
    let mut last_z_vel = 0.;

    // begin :DDD
    // 1 0 Frame { time: 0.0, frame: 0, data: DemoStart }
    let start_framedata = FrameData::DemoStart;
    let start_frame = Frame {
        time,
        frame: 0,
        data: start_framedata,
    };
    entry1.frames.push(start_frame);

    let mut packet_entity_msg = true;

    // insert :DDD
    for (frame_idx, frame) in ghost_info.frames.iter().enumerate() {
        let frametime = override_frametime.unwrap_or({
            if let Some(frametime) = frame.frametime {
                frametime as f32
            } else {
                unreachable!("No frametime");
            }
        });

        let fov = override_fov.unwrap_or(90.);
        let mut vieworigin = frame.origin;

        // vieworigin is not origin
        // we dont know player's state so this is okay
        if let Some(buttons) = frame.buttons {
            if buttons & Buttons::Duck as u32 != 0 {
                vieworigin[2] += 12.;
            } else {
                vieworigin[2] += 17.;
            }
        }

        // buffer because it does so.... not sure the number for now :DDD
        let buffer_framedata = FrameData::DemoBuffer(DemoBufferData {
            buffer: &DEMO_BUFFER_SIZE,
        });
        let buffer_frame = Frame {
            time,
            frame: (frame_idx + 1) as i32,
            data: buffer_framedata,
        };

        // client data
        let clientdata_framedata = FrameData::ClientData(ClientDataData {
            origin: frame.origin.into(),
            viewangles: frame.viewangles.into(),
            weapon_bits: 0,
            fov,
        });
        let clientdata_frame = Frame {
            time,
            frame: (frame_idx + 1) as i32,
            data: clientdata_framedata,
        };

        // netmsg
        let mut new_netmsg_data = NetMsgData::new(DEFAULT_IN_SEQ + frame_idx as i32);
        new_netmsg_data.info.ref_params.vieworg = vieworigin.into();
        new_netmsg_data.info.ref_params.viewangles = frame.viewangles.into();
        new_netmsg_data.info.ref_params.frametime = frametime;
        new_netmsg_data.info.ref_params.time = time;
        new_netmsg_data.info.ref_params.simorg = frame.origin.into();
        new_netmsg_data.info.ref_params.cl_viewangles = frame.viewangles.into();
        new_netmsg_data.info.usercmd.viewangles = frame.viewangles.into();
        // new_netmsg_data.info.movevars.sky_name = hehe; // TODO... DO NOT ASSIGN to &[]
        new_netmsg_data.info.view = vieworigin.into();

        let speed = ((frame.origin[0] - last_pos[0]).powi(2)
            + (frame.origin[1] - last_pos[1]).powi(2))
        .sqrt()
            / frametime;
        let curr_z_vel = (frame.origin[2] - last_pos[2]) / frametime;

        // if speed is less than 150 then increase time_step
        if speed < 150. {
            time_step = STEP_TIME + 0.1;
        }

        let footstep_sound_index_start = game_resource_index_start + 1;

        // play jump sound
        if let Some(buttons) = frame.buttons {
            if buttons & Buttons::Jump as u32 != 0 && curr_z_vel > last_z_vel && speed > 150. {
                let svcsound = SvcSound {
                    flags: bitvec![u8, Lsb0; 1, 1, 1, 0, 0, 0, 0, 0, 0],
                    volume: nbit_num!(128, 8).into(),
                    attenuation: nbit_num!(204, 8).into(),
                    channel: nbit_num!(5, 3),
                    entity_index: nbit_num!(1, 11),
                    sound_index_long: nbit_num!(
                        rand_int_range!(footstep_sound_index_start, footstep_sound_index_start + 3),
                        16
                    )
                    .into(),
                    sound_index_short: None,
                    has_x: true,
                    has_y: true,
                    has_z: true,
                    origin_x: Some(OriginCoord {
                        int_flag: true,
                        fraction_flag: false,
                        is_negative: frame.origin[0].is_sign_negative().into(),
                        int_value: nbit_num!(frame.origin[0].round().abs() as i32, 12).into(),
                        fraction_value: None,
                    }),
                    origin_y: Some(OriginCoord {
                        int_flag: true,
                        fraction_flag: false,
                        is_negative: frame.origin[1].is_sign_negative().into(),
                        int_value: nbit_num!(frame.origin[1].round().abs() as i32, 12).into(),
                        fraction_value: None,
                    }),
                    origin_z: Some(OriginCoord {
                        int_flag: true,
                        fraction_flag: false,
                        is_negative: frame.origin[2].is_sign_negative().into(),
                        int_value: nbit_num!(frame.origin[2].round().abs() as i32, 12).into(),
                        fraction_value: None,
                    }),
                    pitch: bitvec![u8, Lsb0; 1, 0, 0, 0, 0, 0, 0, 0],
                };

                let svcsound_msg = svcsound.write(aux);

                new_netmsg_data.msg = [new_netmsg_data.msg.to_owned(), svcsound_msg]
                    .concat()
                    .leak();
            }
        }
        // play step sound every 0.3 on ground
        if time_step <= 0. && last_pos[2] == frame.origin[2] {
            time_step = STEP_TIME;

            // TODO do all the steps randomly
            let svcsound = SvcSound {
                flags: bitvec![u8, Lsb0; 1, 1, 1, 0, 0, 0, 0, 0, 0],
                volume: nbit_num!(128, 8).into(),
                attenuation: nbit_num!(204, 8).into(),
                channel: nbit_num!(5, 3),
                entity_index: nbit_num!(1, 11),
                sound_index_long: nbit_num!(
                    rand_int_range!(footstep_sound_index_start, footstep_sound_index_start + 3),
                    16
                )
                .into(),
                sound_index_short: None,
                has_x: true,
                has_y: true,
                has_z: true,
                origin_x: Some(OriginCoord {
                    int_flag: true,
                    fraction_flag: false,
                    is_negative: frame.origin[0].is_sign_negative().into(),
                    int_value: nbit_num!(frame.origin[0].round().abs() as i32, 12).into(),
                    fraction_value: None,
                }),
                origin_y: Some(OriginCoord {
                    int_flag: true,
                    fraction_flag: false,
                    is_negative: frame.origin[1].is_sign_negative().into(),
                    int_value: nbit_num!(frame.origin[1].round().abs() as i32, 12).into(),
                    fraction_value: None,
                }),
                origin_z: Some(OriginCoord {
                    int_flag: true,
                    fraction_flag: false,
                    is_negative: frame.origin[2].is_sign_negative().into(),
                    int_value: nbit_num!(frame.origin[2].round().abs() as i32, 12).into(),
                    fraction_value: None,
                }),
                pitch: nbit_num!(1, 8),
            };

            let svcsound_msg = svcsound.write(aux);

            new_netmsg_data.msg = [new_netmsg_data.msg.to_owned(), svcsound_msg]
                .concat()
                .leak();
        }

        if packet_entity_msg {
            new_netmsg_data.msg = [
                packet_entities.to_owned(),
                // delta_packet_entities_byte,
                new_netmsg_data.msg.to_owned(),
            ]
            .concat()
            .leak();

            packet_entity_msg = false;
        }

        if frame_idx % 100 == 0 {
            // let mut delta_packet_entities = delta_packet_entities;
            // println!("{} {}", delta_packet_entities.entity_states.len(), delta_packet_entities.entity_count.to_u32());
            // delta_sequence: nbit_num!(DEFAULT_IN_SEQ & 0xff - 1, 8), // otherwise entity flush happens
            delta_packet_entities.delta_sequence =
                nbit_num!((DEFAULT_IN_SEQ + frame_idx as i32 - 1) & 0xff, 8);
            let delta_packet_entities_byte = delta_packet_entities.write(aux);

            new_netmsg_data.msg = [
                // packet_entities.to_owned(),
                delta_packet_entities_byte,
                new_netmsg_data.msg.to_owned(),
            ]
            .concat()
            .leak();
        }

        let netmsg_framedata = FrameData::NetMsg((NetMsgFrameType::Normal, new_netmsg_data));
        let netmsg_frame = Frame {
            time,
            frame: (frame_idx + 1) as i32,
            data: netmsg_framedata,
        };

        // insert
        entry1
            .frames
            .append(&mut vec![buffer_frame, clientdata_frame, netmsg_frame]);

        time += frametime;
        time_step -= frametime;
        last_pos = frame.origin.into();
        last_z_vel = curr_z_vel;
    }

    // demo section end :DD
    // 1 388 Frame { time: 1.260376, frame: 126, data: NextSection }
    let end_framedata = FrameData::DemoStart;
    let end_frame = Frame {
        time,
        frame: ghost_info.frames.len() as i32,
        data: end_framedata,
    };

    entry1.frames.push(end_frame);
    entry1.frame_count = ghost_info.frames.len() as i32;
}

#[cfg(test)]
mod test {
    use dem::write_demo;

    use super::*;

    #[test]
    fn run() {
        let demo = ghost_to_demo(
            Path::new("/home/khang/gchimp/examples/ghost2dem/rvp.rj.json"),
            Path::new("/home/khang/gchimp/examples/ghost2dem/rvp_tundra-bhop.bsp"),
        );

        write_demo("/home/khang/gchimp/examples/ghost2dem/out.dem", demo).unwrap();
    }
}
