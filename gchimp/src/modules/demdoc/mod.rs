use dem::hldemo::{MoveVars, NetMsgData, NetMsgInfo, RefParams, UserCmd};

pub mod change_map;
pub mod check_doctored;
pub mod ghost2dem;
pub mod kz_stats;
mod utils;

#[macro_export]
macro_rules! wrap_message {
    ($svc:ident, $msg:ident) => {{
        use dem::types::EngineMessage;
        use dem::types::NetMessage;

        let huh = EngineMessage::$svc($msg);
        let hah = NetMessage::EngineMessage(Box::new(huh));
        hah
    }};
}

#[repr(u16)]
pub enum Buttons {
    Attack = 1 << 0,
    Jump = 1 << 1,
    Duck = 1 << 2,
    Forward = 1 << 3,
    Back = 1 << 4,
    Use = 1 << 5,
    Cancel = 1 << 6,
    Left = 1 << 7,
    Right = 1 << 8,
    MoveLeft = 1 << 9,
    MoveRight = 1 << 10,
    Attack2 = 1 << 11,
    Run = 1 << 12,
    Reload = 1 << 13,
    Alt1 = 1 << 14,
    Score = 1 << 15,
}

pub enum ResourceType {
    Sound = 0,
    Skin = 1,
    Model = 2,
    Decal = 3,
    Generic = 4,
    Eventscript = 5,
    World = 6,
}

const VEC_0: [f32; 3] = [0., 0., 0.];
const VIEWHEIGHT: [f32; 3] = [0.0, 0.0, 17.0];
const VIEWPORT: [i32; 4] = [0, 0, 1024, 768];
const SKYNAME: [u8; 32] = [0u8; 32];

pub trait NetMsgDataMethods {
    /// Creates semi-default net message data for CS 1.6
    ///
    /// Recommended to change fields after this. Or just add new method :DDD
    ///
    /// seq: Sequence number for the net message. It is to ensure that the demo won't crash.
    /// Try to use a value that is not `0` for it.
    fn new(seq: i32) -> Self;
}

impl<'a> NetMsgDataMethods for NetMsgData<'a> {
    fn new(seq: i32) -> Self {
        Self {
            info: NetMsgInfo {
                timestamp: 0.0,
                ref_params: RefParams {
                    vieworg: VEC_0,
                    viewangles: VEC_0,
                    forward: VEC_0,
                    right: VEC_0,
                    up: VEC_0,
                    frametime: 0.,
                    time: 0.,
                    intermission: 0,
                    paused: 0,
                    spectator: 0,
                    onground: 0,
                    waterlevel: 0,
                    simvel: VEC_0,
                    simorg: VEC_0,
                    viewheight: VIEWHEIGHT,
                    idealpitch: 0.,
                    cl_viewangles: VEC_0,
                    health: 100,
                    crosshairangle: VEC_0,
                    viewsize: 120.,
                    punchangle: VEC_0,
                    maxclients: 32,
                    viewentity: 1,
                    playernum: 0,
                    max_entities: 6969,
                    demoplayback: 0,
                    hardware: 1,
                    smoothing: 1,
                    ptr_cmd: 0,
                    ptr_movevars: 0,
                    viewport: VIEWPORT,
                    next_view: 0,
                    only_client_draw: 0,
                },
                usercmd: UserCmd {
                    lerp_msec: 9,
                    msec: 10,
                    viewangles: VEC_0,
                    forwardmove: 0.,
                    sidemove: 0.,
                    upmove: 0.,
                    lightlevel: 68,
                    buttons: 0,
                    impulse: 0,
                    weaponselect: 0,
                    impact_index: 0,
                    impact_position: VEC_0,
                },
                movevars: MoveVars {
                    gravity: 800.0,
                    stopspeed: 75.0,
                    maxspeed: 320.,
                    spectatormaxspeed: 500.,
                    accelerate: 5.,
                    airaccelerate: 10.,
                    wateraccelerate: 10.,
                    friction: 4.,
                    edgefriction: 2.,
                    waterfriction: 1.,
                    entgravity: 1.,
                    bounce: 1.,
                    stepsize: 18.,
                    maxvelocity: 2000.,
                    zmax: 409600.,
                    wave_height: 0.,
                    footsteps: 1,
                    sky_name: &SKYNAME, // TODO
                    rollangle: 0.,
                    rollspeed: 0.,
                    skycolor_r: 0.,
                    skycolor_g: 0.,
                    skycolor_b: 0.,
                    skyvec_x: 0.,
                    skyvec_y: 0.,
                    skyvec_z: 0.,
                },
                view: VEC_0,
                viewmodel: 0,
            },
            // To make sure that game doesn't crash, change it like this.
            incoming_sequence: seq,
            incoming_acknowledged: seq - 1,
            incoming_reliable_acknowledged: 1,
            incoming_reliable_sequence: 0,
            outgoing_sequence: seq,
            reliable_sequence: 1,
            last_reliable_sequence: seq - 1,
            msg: &[],
        }
    }
}
