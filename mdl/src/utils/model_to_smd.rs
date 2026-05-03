//! This is mainly for MDL -> MDL
//!
//! Due to how MDL format works, it is rather difficult to parse-write without interpreting MDL format.
//!
//! So, an intermediate data should be in place for ease of conversion. SMD is the format for it.

use std::ffi::CStr;

use crate::{MeshTriangles, Model, Texture, Trivert};

fn trivert_to_smd_vertex(trivert: &Trivert, parent: i32, texture: &Texture) -> smd::Vertex {
    smd::Vertex {
        parent,
        pos: trivert.vertex.as_dvec3(),
        norm: trivert.normal.as_dvec3(),
        uv: [
            trivert.header.s as f64 / texture.dimensions().0 as f64,
            trivert.header.t as f64 / texture.dimensions().1 as f64,
        ]
        .into(),
        source: None,
    }
}

impl Model {
    /// Replaces [`Model.agnostic_mesh`] with derived SMD mesh data from [`Model.meshes`]
    pub fn build_agnostic_data(&mut self, textures: &[Texture]) {
        let mut smd_mesh: Vec<smd::Triangle> = Vec::new();

        for mesh in &self.meshes {
            let curr_texture = &textures[mesh.header.skin_ref as usize];
            let curr_texture_name = CStr::from_bytes_until_nul(&curr_texture.header.name)
                .expect("cannot parse texture name")
                .to_string_lossy()
                .to_string();

            let get_smd_vertex = |v: &Trivert| {
                let parent = self.vertex_info[v.header.vert_index as usize] as i32; // bone idx

                trivert_to_smd_vertex(v, parent, curr_texture)
            };

            for mesh_tri in &mesh.triangles {
                match mesh_tri {
                    MeshTriangles::Strip(triverts) => {
                        // A strip with N triverts has N-2 triangles
                        for i in 0..triverts.len().saturating_sub(2) {
                            let v1 = &triverts[i];
                            let v2 = &triverts[i + 1];
                            let v3 = &triverts[i + 2];

                            let smd_v1 = get_smd_vertex(v1);
                            let smd_v2 = get_smd_vertex(v2);
                            let smd_v3 = get_smd_vertex(v3);

                            if i % 2 == 0 {
                                smd_mesh.push(smd::Triangle {
                                    material: curr_texture_name.clone(),
                                    vertices: vec![smd_v1, smd_v2, smd_v3],
                                });
                            } else {
                                smd_mesh.push(smd::Triangle {
                                    material: curr_texture_name.clone(),
                                    vertices: vec![smd_v2, smd_v1, smd_v3],
                                });
                            }
                        }
                    }
                    MeshTriangles::Fan(triverts) => {
                        if triverts.len() < 3 {
                            continue;
                        }

                        let v_first = &triverts[0];
                        let smd_v_first = get_smd_vertex(v_first);

                        // A fan always uses the first vertex as the pivot
                        for i in 1..triverts.len().saturating_sub(1) {
                            let v2 = &triverts[i];
                            let v3 = &triverts[i + 1];

                            let smd_v2 = get_smd_vertex(v2);
                            let smd_v3 = get_smd_vertex(v3);

                            smd_mesh.push(smd::Triangle {
                                material: curr_texture_name.clone(),
                                vertices: vec![smd_v_first.clone(), smd_v2, smd_v3],
                            });
                        }
                    }
                }
            }
        }

        self.agnostic_mesh = Some(smd_mesh);
    }
}
