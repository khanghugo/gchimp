use glam::DVec3;

use crate::types::{Smd, Triangle, Vertex};

impl Vertex {
    pub fn bad_hash(&self) -> String {
        format!("{}{}", self.bad_pos_hash(), self.bad_norm_hash())
    }

    pub fn bad_pos_hash(&self) -> String {
        let zzz = self.pos;
        format!("{}{}{}", zzz.x, zzz.y, zzz.z)
    }

    pub fn bad_norm_hash(&self) -> String {
        let norm = self.norm;
        format!("{}{}{}", norm.x, norm.y, norm.z)
    }

    pub fn bad_uv_hash(&self) -> String {
        let uv = self.uv;
        format!("{}{}", uv.x, uv.y)
    }
}

pub trait SmdAffineTransformation {
    fn translate(&mut self, value: DVec3);
    fn rotate(&mut self, value: DVec3);
    fn scale(&mut self, value: f64);
}

impl SmdAffineTransformation for Smd {
    fn translate(&mut self, value: DVec3) {
        self.triangles.translate(value);
    }

    fn rotate(&mut self, value: DVec3) {
        self.triangles.rotate(value);
    }

    fn scale(&mut self, value: f64) {
        self.triangles.scale(value);
    }
}

impl SmdAffineTransformation for Vec<Triangle> {
    fn translate(&mut self, value: DVec3) {
        self.as_mut_slice().translate(value);
    }

    fn rotate(&mut self, value: DVec3) {
        self.as_mut_slice().rotate(value);
    }

    fn scale(&mut self, value: f64) {
        self.as_mut_slice().scale(value);
    }
}

impl SmdAffineTransformation for &mut [Triangle] {
    fn translate(&mut self, value: DVec3) {
        for tri in self.iter_mut() {
            for v in &mut tri.vertices {
                v.pos += value;
            }
        }
    }

    fn rotate(&mut self, value: DVec3) {
        // Create a rotation matrix from Euler angles (XYZ order)
        let rot_mat = glam::DMat3::from_euler(glam::EulerRot::XYZ, value.x, value.y, value.z);

        for tri in self.iter_mut() {
            for v in &mut tri.vertices {
                // Rotate position
                v.pos = rot_mat * v.pos;
                // Rotate normal and re-normalize to ensure it's still a unit vector
                v.norm = (rot_mat * v.norm).normalize();
            }
        }
    }

    fn scale(&mut self, value: f64) {
        for tri in self.iter_mut() {
            for v in &mut tri.vertices {
                v.pos *= value;
            }
        }
    }
}
