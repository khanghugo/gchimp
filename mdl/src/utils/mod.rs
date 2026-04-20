use glam::{Mat3, Vec3};

use crate::{AnimValues, Bone, Header, MeshTriangles, SequenceGroup, Trivert};
use crate::{Mdl, Sequence, SequenceHeader};

mod model_to_smd;

impl Bone {
    pub fn new_empty() -> Self {
        Self {
            // "root"
            name: [
                114, 111, 111, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0,
            ],
            parent: -1,
            flags: 0,
            bone_controller: [-1; 6],
            value: [0.; 6],
            scale: [0.; 6],
        }
    }
}

impl Sequence {
    pub fn new_empty() -> Self {
        Self {
            header: SequenceHeader {
                // "idle"
                label: [
                    105, 100, 108, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                ],
                fps: 30.,
                num_blends: 1,
                blend_end: [1., 0.],
                ..Default::default()
            },
            anim_blends: vec![vec![[
                AnimValues(vec![0]),
                AnimValues(vec![0]),
                AnimValues(vec![0]),
                AnimValues(vec![0]),
                AnimValues(vec![0]),
                AnimValues(vec![0]),
            ]]],
        }
    }
}

impl SequenceGroup {
    pub fn new_empty() -> Self {
        Self {
            // "default"
            label: [
                100, 101, 102, 97, 117, 108, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            name: [0; 64],
            unused1: 0,
            unused2: 0,
        }
    }
}

impl Default for Header {
    fn default() -> Self {
        Self {
            id: 1414743113, // TSDI
            version: 10,
            name: [0; 64],
            length: 0,
            eye_position: Default::default(),
            min: Default::default(),
            max: Default::default(),
            bbmin: Default::default(),
            bbmax: Default::default(),
            flags: Default::default(),
            num_bones: Default::default(),
            bone_index: Default::default(),
            num_bone_controllers: Default::default(),
            bone_controller_index: Default::default(),
            num_hitboxes: Default::default(),
            hitbox_index: Default::default(),
            num_seq: Default::default(),
            seq_index: Default::default(),
            num_seq_group: Default::default(),
            seq_group_index: Default::default(),
            num_textures: Default::default(),
            texture_index: Default::default(),
            texture_data_index: Default::default(),
            num_skin_ref: Default::default(),
            num_skin_families: Default::default(),
            skin_index: Default::default(),
            num_bodyparts: Default::default(),
            bodypart_index: Default::default(),
            num_attachments: Default::default(),
            attachment_index: Default::default(),
            sound_table: Default::default(),
            sound_index: Default::default(),
            sound_groups: Default::default(),
            sound_group_index: Default::default(),
            num_transitions: Default::default(),
            transition_index: Default::default(),
        }
    }
}

impl Mdl {
    pub fn new_empty() -> Self {
        Self {
            header: Header {
                // this is redundant
                num_seq: 1,
                num_seq_group: 1,
                ..Default::default()
            },
            sequences: vec![Sequence::new_empty()],
            textures: vec![],
            bodyparts: vec![],
            bones: vec![Bone::new_empty()],
            bone_controllers: vec![],
            hitboxes: vec![],
            sequence_groups: vec![SequenceGroup::new_empty()],
            skin_families: vec![],
            attachments: vec![],
            transitions: vec![],
        }
    }

    // TODO this only works for single skin family
    fn build_skin_families(&mut self) {
        self.skin_families = vec![(0..(self.textures.len() as i16)).collect()];
    }

    /// In order to export the model file, must invoke this function before exporting.
    pub fn rebuild_data_for_export(&mut self) {
        // only rebuild mesh if agnostic mesh is all empty
        if self.bodyparts.iter().all(|bodypart| {
            bodypart
                .models
                .iter()
                .all(|model| model.agnostic_mesh.is_none())
        }) {
            self.bodyparts.iter_mut().for_each(|bodypart| {
                bodypart
                    .models
                    .iter_mut()
                    .for_each(|model| model.build_agnostic_data(&self.textures))
            });
        }

        // skinfamilies must match textures count
        self.build_skin_families();
    }

    /// Returns model triangle count at any typical given moment
    pub fn triangle_count(&self) -> usize {
        self.bodyparts.iter().fold(0, |acc, e| {
            acc + e
                .models
                .iter()
                .next() // count the first model cuz not all of them are in-used
                .map(|x| {
                    x.meshes
                        .iter()
                        .fold(0, |acc2, e2| acc2 + e2.header.num_tris.abs() as usize)
                })
                .unwrap_or(0)
        })
    }

    pub fn set_name(&mut self, name: &str) {
        self.header.name[..name.len().min(64)]
            .copy_from_slice(&name.as_bytes()[..(name.len().min(64))]);
    }
}

pub trait TrivertAffineTransformation {
    fn translate(&mut self, value: Vec3);
    fn rotate(&mut self, value: Vec3);
    fn scale(&mut self, value: f32);
    fn transform_mat3(&mut self, value: Mat3);
}

impl TrivertAffineTransformation for MeshTriangles {
    fn translate(&mut self, value: Vec3) {
        match self {
            MeshTriangles::Strip(triverts) | MeshTriangles::Fan(triverts) => {
                triverts.translate(value);
            }
        }
    }

    fn rotate(&mut self, value: Vec3) {
        match self {
            MeshTriangles::Strip(triverts) | MeshTriangles::Fan(triverts) => {
                triverts.rotate(value);
            }
        }
    }

    fn scale(&mut self, value: f32) {
        match self {
            MeshTriangles::Strip(triverts) | MeshTriangles::Fan(triverts) => {
                triverts.scale(value);
            }
        }
    }

    fn transform_mat3(&mut self, value: Mat3) {
        match self {
            MeshTriangles::Strip(triverts) | MeshTriangles::Fan(triverts) => {
                triverts.transform_mat3(value);
            }
        }
    }
}

impl TrivertAffineTransformation for Vec<Trivert> {
    fn translate(&mut self, value: Vec3) {
        self.as_mut_slice().translate(value);
    }

    fn rotate(&mut self, value: Vec3) {
        self.as_mut_slice().rotate(value);
    }

    fn scale(&mut self, value: f32) {
        self.as_mut_slice().scale(value);
    }

    fn transform_mat3(&mut self, value: Mat3) {
        self.as_mut_slice().transform_mat3(value);
    }
}

impl TrivertAffineTransformation for &mut [Trivert] {
    fn translate(&mut self, value: Vec3) {
        for tri in self.iter_mut() {
            tri.vertex += value;
        }
    }

    fn rotate(&mut self, value: Vec3) {
        let rot_mat = glam::Mat3::from_euler(glam::EulerRot::ZYX, value.z, value.y, value.x);

        for tri in self.iter_mut() {
            tri.vertex = rot_mat * tri.vertex;
            tri.normal = (rot_mat * tri.normal).normalize();
        }
    }

    fn scale(&mut self, value: f32) {
        for tri in self.iter_mut() {
            tri.vertex *= value;
        }
    }

    fn transform_mat3(&mut self, value: glam::Mat3) {
        for tri in self.iter_mut() {
            tri.vertex = value * tri.vertex;
            tri.normal = (value * tri.normal).normalize();
        }
    }
}
