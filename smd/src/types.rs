use glam::{DVec2, DVec3};

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: i32,
    pub bone_name: String,
    pub parent: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Skeleton {
    pub time: i32,
    pub bones: Vec<BonePos>,
}

impl Skeleton {
    pub fn new_basic() -> Self {
        Self {
            time: 0,
            bones: vec![BonePos {
                id: 0,
                pos: [0., 0., 0.].into(),
                rot: [0., 0., 0.].into(),
            }],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BonePos {
    pub id: i32,
    pub pos: DVec3,
    pub rot: DVec3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Triangle {
    pub material: String,
    pub vertices: Vec<Vertex>,
}

impl Triangle {
    pub fn flip_winding_order_mut(&mut self) {
        self.vertices.swap(0, 1);
    }

    // opposite of /home/khang/gchimp/studiomdl/src/types.rs Mesh::normalized_smd_uv_to_mdl_uv
    // which is exact same thing because of math
    pub fn mdl_uv_to_normalized_smd_uv(&mut self) {
        self.vertices
            .iter_mut()
            .for_each(|vert| vert.uv = (vert.uv.x, 1.0 - vert.uv.y).into());
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub parent: i32,
    pub pos: DVec3,
    pub norm: DVec3,
    pub uv: DVec2,
    /// Optional for Source
    pub source: Option<VertexSourceInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VertexSourceInfo {
    pub links: i32,
    pub bone: Option<i32>,
    pub weight: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VertexAnim {
    pub time: i32,
    pub vertices: Vec<VertexAnimPos>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VertexAnimPos {
    pub id: i32,
    pub pos: DVec3,
    pub norm: DVec3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Smd {
    pub version: i32,
    pub nodes: Vec<Node>,
    pub skeleton: Vec<Skeleton>,
    // triangles is optional because sequence file does not have triangles apparently.
    pub triangles: Vec<Triangle>,
    /// Optional for Source
    pub vertex_anim: Vec<VertexAnim>,
}
