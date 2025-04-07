use std::collections::HashMap;

use glam::Vec3;
use wad::types::MipTex;

use nom::IResult as _IResult;

use crate::constants::MAX_MAP_HULLS;

pub type IResult<'a, T> = _IResult<&'a [u8], T>;
pub type SResult<'a, T> = _IResult<&'a str, T>;

#[derive(Debug)]
pub struct LumpHeader {
    pub offset: i32,
    pub length: i32,
}

pub type Entity = HashMap<String, String>;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum PlaneType {
    X = 0,
    Y = 1,
    Z = 2,
    AnyX = 3,
    AnyY = 4,
    AnyZ = 5,
}

impl TryFrom<i32> for PlaneType {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if !(0..=5).contains(&value) {
            return Err("Not a valid plane type");
        }

        Ok(match value {
            0 => Self::X,
            1 => Self::Y,
            2 => Self::Z,
            3 => Self::AnyX,
            4 => Self::AnyY,
            5 => Self::AnyZ,
            _ => unreachable!(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
    pub type_: PlaneType,
}

impl Plane {
    /// Returns the plane equation
    pub fn equation(&self) -> String {
        format!(
            "{}x {}{}y {}{}z = {}",
            self.normal.x.abs(),
            if self.normal.y.is_sign_positive() {
                "+"
            } else {
                ""
            },
            self.normal.y.abs(),
            if self.normal.z.is_sign_positive() {
                "+"
            } else {
                ""
            },
            self.normal.z.abs(),
            self.distance
        )
    }

    pub fn flip(&self) -> Self {
        Self {
            normal: -self.normal,
            distance: -self.distance,
            type_: self.type_,
        }
    }
}

pub type Texture = MipTex;
pub type Vertex = Vec3;
// pub type BspVis = todo!();

#[derive(Debug)]
pub struct Node {
    pub plane: u32,
    pub children: [i16; 2],
    pub mins: [i16; 3],
    pub maxs: [i16; 3],
    pub first_face: u16,
    pub face_count: u16,
}

#[derive(Debug)]
pub struct TexInfo {
    pub u: Vec3,
    pub u_offset: f32,
    pub v: Vec3,
    pub v_offset: f32,
    pub texture_index: u32,
    pub flags: u32,
}

#[derive(Debug)]
pub struct Face {
    pub plane: u16,
    pub side: u16,
    pub first_edge: i32,
    pub edge_count: u16,
    pub texinfo: u16,
    pub styles: [u8; 4],
    pub lightmap_offset: i32,
}

pub type LightMap = Vec<[u8; 3]>;

#[derive(Debug)]
pub struct ClipNode {
    pub plane: i32,
    pub children: [i16; 2],
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum LeafContent {
    ContentsEmpty = -1,
    ContentsSolid = -2,
    ContentsWater = -3,
    ContentsSlime = -4,
    ContentsLava = -5,
    ContentsSky = -6,
    ContentsOrigin = -7,
    ContentsClip = -8,
    ContentsCurrent0 = -9,
    ContentsCurrent90 = -10,
    ContentsCurrent180 = -11,
    ContentsCurrent270 = -12,
    ContentsCurrentUp = -13,
    ContentsCurrentDown = -14,
    ContentsTranslucent = -15,
}

impl TryFrom<i32> for LeafContent {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if !(-15..=-1).contains(&value) {
            return Err("Not a valid LeafContent value");
        }

        Ok(match value {
            -1 => Self::ContentsEmpty,
            -2 => Self::ContentsSolid,
            -3 => Self::ContentsWater,
            -4 => Self::ContentsSlime,
            -5 => Self::ContentsLava,
            -6 => Self::ContentsSky,
            -7 => Self::ContentsOrigin,
            -8 => Self::ContentsClip,
            -9 => Self::ContentsCurrent0,
            -10 => Self::ContentsCurrent90,
            -11 => Self::ContentsCurrent180,
            -12 => Self::ContentsCurrent270,
            -13 => Self::ContentsCurrentUp,
            -14 => Self::ContentsCurrentDown,
            -15 => Self::ContentsTranslucent,
            _ => unreachable!(),
        })
    }
}

#[derive(Debug)]
pub struct Leaf {
    pub contents: LeafContent,
    pub vis_offset: i32,
    pub mins: [i16; 3],
    pub maxs: [i16; 3],
    pub first_mark_surface: u16,
    pub mark_surface_count: u16,
    pub ambient_levels: [u8; 4],
}

pub type MarkSurface = u16;
pub type Edge = [u16; 2];
pub type SurfEdge = i32;

#[derive(Debug)]
pub struct Model {
    pub mins: Vec3,
    pub maxs: Vec3,
    pub origin: Vec3,
    pub head_nodes: [i32; MAX_MAP_HULLS],
    pub vis_leaves_count: i32,
    pub first_face: i32,
    pub face_count: i32,
}

#[derive(Debug)]
pub struct Bsp {
    pub entities: Vec<Entity>,
    pub planes: Vec<Plane>,
    pub textures: Vec<Texture>,
    pub vertices: Vec<Vertex>,
    // TODO vis
    pub visibility: Vec<u8>,
    pub nodes: Vec<Node>,
    pub texinfo: Vec<TexInfo>,
    pub faces: Vec<Face>,
    pub lightmap: LightMap,
    pub clipnodes: Vec<ClipNode>,
    pub leaves: Vec<Leaf>,
    pub mark_surfaces: Vec<MarkSurface>,
    pub edges: Vec<Edge>,
    pub surf_edges: Vec<SurfEdge>,
    pub models: Vec<Model>,
}
