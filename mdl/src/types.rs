use bitflags::bitflags;
use glam::Vec3;

pub const VEC3_T_SIZE: usize = 3 * 4;

pub struct Mdl {
    pub header: Header,
    pub sequences: Vec<Sequence>,
    pub textures: Vec<Texture>,
    pub bodyparts: Vec<Bodypart>,
    pub bones: Vec<Bone>,
    pub bone_controllers: Vec<BoneController>,
    pub hitboxes: Vec<Hitbox>,
    pub sequence_groups: Vec<SequenceGroup>,
    pub skin_families: SkinFamilies,
    pub attachments: Vec<Attachment>,
}

pub struct Header {
    pub id: i32,
    pub version: i32,
    pub name: [u8; 64],
    pub length: i32,
    pub eye_position: Vec3,
    pub min: Vec3,
    pub max: Vec3,
    pub bbmin: Vec3,
    pub bbmax: Vec3,
    pub flags: i32,
    pub num_bones: i32,
    pub bone_index: i32,
    pub num_bone_controllers: i32,
    pub bone_controller_index: i32,
    pub num_hitboxes: i32,
    pub hitbox_index: i32,
    pub num_seq: i32,
    pub seq_index: i32,
    pub num_seq_group: i32,
    pub seq_group_index: i32,
    pub num_textures: i32,
    pub texture_index: i32,
    pub texture_data_index: i32,
    pub num_skin_ref: i32,
    pub num_skin_families: i32,
    pub skin_index: i32,
    pub num_bodyparts: i32,
    pub bodypart_index: i32,
    pub num_attachments: i32,
    pub attachment_index: i32,
    pub sound_table: i32,
    pub sound_index: i32,
    pub sound_groups: i32,
    pub sound_group_index: i32,
    pub num_transitions: i32,
    pub transition_index: i32,
}

bitflags! {
    #[derive(Debug)]
    pub struct SequenceFlag: i32 {
        const LOOPING = 1 << 0;
    }
}

#[derive(Debug)]
pub struct SequenceHeader {
    pub label: [u8; 32],
    pub fps: f32,
    pub flags: SequenceFlag,
    pub activity: i32,
    pub act_weight: i32,
    pub num_events: i32,
    pub event_index: i32,
    pub num_frames: i32,
    pub num_pivots: i32,
    pub pivot_index: i32,
    pub motion_type: i32,
    pub motion_bone: i32,
    pub linear_movement: Vec3,
    pub auto_move_pos_index: i32,
    pub auto_move_angle_index: i32,
    pub bbmin: Vec3,
    pub bbmax: Vec3,
    pub num_blends: i32,
    pub anim_index: i32,
    pub blend_type: [i32; 2],
    pub blend_start: [f32; 2],
    pub blend_end: [f32; 2],
    pub blend_parent: i32,
    pub seq_group: i32,
    pub entry_node: i32,
    pub exit_node: i32,
    pub node_flags: i32,
    pub next_seq: i32,
}

/// [[[animation value; frame count]; 6 motion types]; bone count]
///
/// Number is i16, not u16 because we want signed.
pub type Blend = Vec<BlendBone>;
pub type BlendBone = [AnimValues; 6];
pub type AnimValues = Vec<i16>;

#[derive(Debug)]
pub struct Sequence {
    pub header: SequenceHeader,
    /// `[[[[short animation value; frame count]; 6 motion types]; bone count]; blend count]`
    pub anim_blends: Vec<Blend>,
}

bitflags! {
    pub struct TextureFlag: i32 {
        const FLATSHADE = 1;
        const CHROME = 1 << 1;
        const FULLBRIGHT = 1 << 2;
        const NOMIPS = 1 << 3;
        const ALPHA = 1 << 4;
        const ADDITIVE = 1 << 5;
        const MASKED = 1 << 6;
    }
}

pub struct TextureHeader {
    pub name: [u8; 64],
    pub flags: TextureFlag,
    pub width: i32,
    pub height: i32,
    pub index: i32,
}

pub const PALETTE_COUNT: usize = 256;

pub struct Texture {
    pub header: TextureHeader,
    pub image: Vec<u8>,
    pub palette: [[u8; 3]; PALETTE_COUNT],
}

impl Texture {
    pub fn rgb8_bytes(&self) -> Vec<u8> {
        self.image
            .iter()
            .flat_map(|&pixel| self.palette[pixel as usize])
            .collect()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.header.width as u32, self.header.height as u32)
    }
}

pub struct BodypartHeader {
    pub name: [u8; 64],
    pub num_models: i32,
    pub base: i32,
    pub model_index: i32,
}

pub struct Bodypart {
    pub header: BodypartHeader,
    pub models: Vec<Model>,
}

pub struct ModelHeader {
    pub name: [u8; 64],
    pub type_: i32,
    pub bounding_radius: f32,
    pub num_mesh: i32,
    pub mesh_index: i32,
    pub num_verts: i32,
    pub vert_info_index: i32,
    pub vert_index: i32,
    pub num_norms: i32,
    pub norm_info_index: i32,
    pub norm_index: i32,
    pub num_groups: i32,
    pub group_index: i32,
}

pub struct Model {
    pub header: ModelHeader,
    pub meshes: Vec<Mesh>,
    /// > """vertexinfoindex is the offset to an array of int.
    /// This array is the same size as numverts and maps each vertex position to a bone index.
    /// This bone index is used to tell which bone affects this vertex position."""
    ///
    /// But it is actually u8 type, not i32
    pub vertex_info: Vec<u8>,
}

pub struct MeshHeader {
    pub num_tris: i32,
    pub tri_index: i32,
    pub skin_ref: i32,
    pub num_norms: i32,
    pub norm_index: i32,
}

pub struct Mesh {
    pub header: MeshHeader,
    /// Each [`MeshTriangles`] could be multiple triangles.
    ///
    /// So, this `triangle` field stores arrays of triangles.
    pub triangles: Vec<MeshTriangles>,
}

pub enum MeshTriangles {
    Strip(Vec<Trivert>),
    Fan(Vec<Trivert>),
}

// impl Mesh {
//     pub fn store_order(&self) -> TrivertStoreOrder {
//         if self.header.num_tris.is_positive() {
//             TrivertStoreOrder::Strip
//         } else {
//             TrivertStoreOrder::Fan
//         }
//     }

//     pub fn is_fan(&self) -> bool {
//         matches!(self.store_order(), TrivertStoreOrder::Fan)
//     }

//     pub fn is_strip(&self) -> bool {
//         matches!(self.store_order(), TrivertStoreOrder::Strip)
//     }
// }

#[derive(Debug, Clone, Copy)]
pub struct TrivertHeader {
    pub vert_index: i16,
    pub norm_index: i16,
    pub s: i16,
    pub t: i16,
}

#[derive(Debug, Clone, Copy)]
pub struct Trivert {
    pub header: TrivertHeader,
    pub vertex: Vec3,
    pub normal: Vec3,
}

#[derive(Debug)]
pub struct Bone {
    pub name: [u8; 32],
    pub parent: i32,
    pub flags: i32,
    pub bone_controller: [i32; 6],
    pub value: [f32; 6],
    pub scale: [f32; 6],
}

pub struct BoneController {
    pub bone: i32,
    pub type_: i32,
    pub start: f32,
    pub end: f32,
    pub rest: i32,
    pub index: i32,
}

pub struct Hitbox {
    pub bone: i32,
    pub group: i32,
    pub bbmin: Vec3,
    pub bbmax: Vec3,
}

pub struct SequenceGroup {
    pub label: [u8; 32],
    pub name: [u8; 64],
    pub unused1: i32,
    pub unused2: i32,
}

pub type SkinFamilies = Vec<Vec<i16>>;

pub struct Attachment {
    pub name: [u8; 32],
    pub type_: i32,
    pub bone: i32,
    pub org: Vec3,
    pub vectors: [Vec3; 3],
}
