//! This is mainly for MDL -> MDL
//!
//! Due to how MDL format works, it is rather difficult to parse-write without interpreting MDL format.
//!
//! So, an intermediate data should be in place for ease of conversion. SMD is the format for it.

use crate::{Mdl, MeshTriangles, Model, Texture, Trivert};
use std::ffi::CStr;

// not quite SMD vertex equivalent
// must do some post fix
// check /home/khang/gchimp/studiomdl/src/types.rs Mesh::fix_uv
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
    // BIG TODO: smd and mdl has different uv and winding order
    // currently, studiomdl module has to fix that difference
    // eventually, this piece of code will end up in studiomdl crate
    // by then, remember that this function should flip the UV to correctly extract smd mesh
    pub fn build_agnostic_data(&mut self, textures: &[Texture]) {
        self.agnostic_mesh = Some(model_to_triangles(&self, textures));
    }
}

pub fn mdl_to_meshes(mdl: &Mdl) -> Vec<(String, Vec<smd::Triangle>)> {
    let arr_to_string = |x: &[u8]| {
        CStr::from_bytes_until_nul(x)
            .expect("model has empty texture name")
            .to_str()
            .unwrap()
            .to_string()
    };

    mdl.bodyparts
        .iter()
        .flat_map(|bodypart| {
            bodypart
                .models
                .iter()
                .map(|model| {
                    (
                        arr_to_string(&model.header.name),
                        model_to_triangles(model, &mdl.textures),
                    )
                })
                .collect::<Vec<(String, Vec<smd::Triangle>)>>()
        })
        .collect()
}

fn model_to_triangles(model: &Model, textures: &[Texture]) -> Vec<smd::Triangle> {
    let mut smd_mesh: Vec<smd::Triangle> = Vec::new();

    for mesh in &model.meshes {
        let curr_texture = &textures[mesh.header.skin_ref as usize];
        let curr_texture_name = CStr::from_bytes_until_nul(&curr_texture.header.name)
            .expect("cannot parse texture name")
            .to_string_lossy()
            .to_string();

        let get_smd_vertex = |v: &Trivert| {
            let parent = model.vertex_info[v.header.vert_index as usize] as i32; // bone idx

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

    smd_mesh
}
