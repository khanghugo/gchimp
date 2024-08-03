use std::mem;

use crate::types::LumpHeader;

pub const BSP_VERSION: i32 = 30;

// BSPLUMP
pub const LUMP_ENTITIES: usize = 0;
pub const LUMP_PLANES: usize = 1;
pub const LUMP_TEXTURES: usize = 2;
pub const LUMP_VERTICES: usize = 3;
pub const LUMP_VISIBILITY: usize = 4;
pub const LUMP_NODES: usize = 5;
pub const LUMP_TEXINFO: usize = 6;
pub const LUMP_FACES: usize = 7;
pub const LUMP_LIGHTING: usize = 8;
pub const LUMP_CLIPNODES: usize = 9;
pub const LUMP_LEAVES: usize = 10;
pub const LUMP_MARKSURFACES: usize = 11;
pub const LUMP_EDGES: usize = 12;
pub const LUMP_SURFEDGES: usize = 13;
pub const LUMP_MODELS: usize = 14;
pub const HEADER_LUMPS: usize = 15;

// Max values
pub const MAX_MAP_HULLS: usize = 4;
// pub const MAX_MAP_MODELS: usize = 400;
// pub const MAX_MAP_BRUSHES: usize = 4096;
// pub const MAX_MAP_ENTITIES: usize = 1024;
// pub const MAX_MAP_ENTSTRING: usize = 128 * 1024;
// pub const MAX_MAP_PLANES: usize = 32767;
// pub const MAX_MAP_NODES: usize = 32767;
// pub const MAX_MAP_CLIPNODES: usize = 32767;
// pub const MAX_MAP_LEAFS: usize = 8192;
// pub const MAX_MAP_VERTS: usize = 65535;
// pub const MAX_MAP_FACES: usize = 65535;
// pub const MAX_MAP_MARKSURFACES: usize = 65535;
// pub const MAX_MAP_TEXINFO: usize = 8192;
// pub const MAX_MAP_EDGES: usize = 256000;
// pub const MAX_MAP_SURFEDGES: usize = 512000;
// pub const MAX_MAP_TEXTURES: usize = 512;
// pub const MAX_MAP_MIPTEX: usize = 0x200000;
// pub const MAX_MAP_LIGHTING: usize = 0x200000;
// pub const MAX_MAP_VISIBILITY: usize = 0x200000;
// pub const MAX_MAP_PORTALS: usize = 65536;

pub const HEADER_LUMP_SIZE: usize = mem::size_of::<LumpHeader>();
