use crate::types::{Smd, Vertex};

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

impl Smd {
    pub fn translate(&mut self) {}
}
