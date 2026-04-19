use std::collections::HashMap;

use byte_writer::ByteWriter;
use glam::Vec3;

use crate::{
    Bodypart, BodypartHeader, MeshHeader, Model, ModelHeader, Texture,
    error::MdlError,
    writer::{
        WriteToWriter,
        impl_trait::{WriteToWriterBodyparts, WriteToWriterModel, WriteToWriterModels},
    },
};

impl WriteToWriterBodyparts for &[Bodypart] {
    fn write_to_writer(&self, writer: &mut ByteWriter, textures: &[Texture]) -> usize {
        // write all models first
        let model_offsets: Vec<usize> = self
            .iter()
            .map(|bodypart| {
                let res = bodypart.models.as_slice().write_to_writer(writer, textures);
                writer.align_size(4);
                res
            })
            .collect();

        // write all headers then
        let header_offset = writer.get_offset();

        self.iter()
            .zip(model_offsets)
            .for_each(|(bodypart, model_offset)| {
                let BodypartHeader {
                    name,
                    num_models: _,
                    base,
                    model_index: _,
                } = bodypart.header;

                writer.append_u8_slice(name.as_slice());
                writer.append_i32(bodypart.models.len() as i32);
                // should always be 1, please
                writer.append_i32(base);
                writer.append_i32(model_offset as i32);
            });

        writer.align_size(4);

        header_offset
    }
}

impl WriteToWriterModel for Model {
    fn write_to_writer(&self, writer: &mut ByteWriter, textures: &[Texture]) -> ModelHeader {
        // if mesh data is not built, fail
        let smd_triangles = self
            .agnostic_mesh
            .as_ref()
            .ok_or(MdlError::AgnosticMeshNotBuilt)
            .expect("agnostic data not populated");

        // rebuilding smd data to mdl data

        // partition triangles with the same materials to from separated meshes
        let mut groups: HashMap<String, Vec<&smd::Triangle>> = HashMap::new();
        for tri in smd_triangles {
            groups.entry(tri.material.clone()).or_default().push(tri);
        }

        // texture -> skin_ref lookup
        let tex_lookup: HashMap<String, i32> = textures
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let name = std::ffi::CStr::from_bytes_until_nul(&t.header.name)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                (name, i as i32)
            })
            .collect();

        // build triangle runs
        let mut unique_verts: Vec<Vec3> = Vec::new(); // actual data
        let mut unique_norms: Vec<Vec3> = Vec::new(); // actual data
        let mut unique_verts_bone_info: Vec<u8> = Vec::new(); // bone idx for vert
        let mut unique_norms_bone_info: Vec<u8> = Vec::new(); // bone idx for norm

        let mut unique_vertex_map = HashMap::new(); // vertex data hash to vertex offset

        let mut mesh_entries = Vec::new();

        for (material, tris) in groups {
            let skin_ref = *tex_lookup.get(&material).unwrap_or(&0);
            let texture = &textures[skin_ref as usize];

            // unique normal is stored per mesh, not per model, very cool, FUCKFUCKFUCKFUCKFUCKFUCKFUCK
            let mut unique_normal_map = HashMap::new();

            // ((vert, norm) (u, v))
            // all indices basically stores the vertex index buffer aka how many vertices
            let mut all_indices = Vec::new();

            // need to track mesh count differently so data is correct
            let mesh_norm_index_start = unique_norms.len() as i32;
            let mut mesh_num_norms = 0;

            for tri in tris {
                // ((vert, norm) (u, v))
                let mut tri_indices = [((0i16, 0), (0, 0)); 3];
                for (i, v) in tri.vertices.iter().enumerate() {
                    let vert_key = v.bad_pos_hash();
                    let norm_key = v.bad_norm_hash();

                    let (s, t) = (
                        (v.uv.x * texture.dimensions().0 as f64).round() as i16,
                        (v.uv.y * texture.dimensions().1 as f64).round() as i16,
                    );

                    let found_vertex = unique_vertex_map.entry(vert_key).or_insert_with(|| {
                        let idx = unique_verts.len() as i16;
                        unique_verts.push(v.pos.as_vec3());
                        unique_verts_bone_info.push(v.parent as u8);
                        idx
                    });

                    let found_normal = unique_normal_map.entry(norm_key).or_insert_with(|| {
                        let idx = unique_norms.len() as i16;
                        unique_norms.push(v.norm.as_vec3());
                        unique_norms_bone_info.push(v.parent as u8);
                        mesh_num_norms += 1;
                        idx
                    });

                    tri_indices[i] = ((*found_vertex, *found_normal), (s, t));
                }
                all_indices.push(tri_indices);
            }
            mesh_entries.push((skin_ref, all_indices, mesh_norm_index_start, mesh_num_norms));
        }

        // write to file now
        // for now, just write file anywhere possible and then use offset to access

        // write vertex info aka bone of a vertex
        let vert_info_index = writer.get_offset();
        writer.append_u8_slice(&unique_verts_bone_info);
        writer.align_size(4);

        let norm_info_index = writer.get_offset();
        writer.append_u8_slice(&unique_norms_bone_info);
        writer.align_size(4);

        // write all the unique vertices and normals
        let vert_index = writer.get_offset();
        for v in &unique_verts {
            writer.append_f32_slice(&v.to_array());
        }

        let norm_index = writer.get_offset();
        for n in &unique_norms {
            writer.append_f32_slice(&n.to_array());
        }

        // write the triangle runs aka mesh
        let mut tri_run_offsets = Vec::new();

        for (_, mesh_tris, _, _) in &mesh_entries {
            tri_run_offsets.push(writer.get_offset());
            for tri in mesh_tris {
                writer.append_i16(3); // always output as strip because we're livinging 2020
                for &((vert, norm), (s, t)) in tri {
                    writer.append_i16(vert); // vert_index
                    writer.append_i16(norm); // norm_index
                    writer.append_i16(s); // s
                    writer.append_i16(t); // t
                }
            }
            writer.append_i16(0); // End of this mesh's triangle runs
        }

        // write mesh headers
        let mesh_index = writer.get_offset();
        for (i, (skin_ref, mesh_tris, mesh_norm_index_start, mesh_num_norms)) in
            mesh_entries.iter().enumerate()
        {
            let curr_mesh_index = writer.get_offset();

            writer.append_i32(mesh_tris.len() as i32); // num_tris
            writer.append_i32(tri_run_offsets[i] as i32); // tri_index
            writer.append_i32(*skin_ref); // skin_ref

            // use the actual number rather than the unused number
            writer.append_i32(*mesh_num_norms); // num_norms, actually used and can crash HLAM
            writer.append_i32(*mesh_norm_index_start); // norm_index, array index like tri_index, can crash HLAM

            assert_eq!(
                writer.get_offset() - curr_mesh_index,
                size_of::<MeshHeader>()
            );
        }

        ModelHeader {
            name: self.header.name,
            type_: self.header.type_,
            bounding_radius: self.header.bounding_radius,
            num_mesh: mesh_entries.len() as i32,
            mesh_index: mesh_index as i32,
            num_verts: unique_verts.len() as i32,
            vert_info_index: vert_info_index as i32,
            vert_index: vert_index as i32,
            num_norms: unique_norms.len() as i32,
            norm_info_index: norm_info_index as i32,
            norm_index: norm_index as i32,
            num_groups: self.header.num_groups,
            group_index: self.header.group_index,
        }
    }
}

impl WriteToWriterModels for &[Model] {
    fn write_to_writer(&self, writer: &mut ByteWriter, textures: &[Texture]) -> usize {
        let headers: Vec<ModelHeader> = self
            .iter()
            .map(|model| model.write_to_writer(writer, textures))
            .collect();

        let header_offsets = headers.as_slice().write_to_writer(writer);

        // align data for vec type
        writer.align_size(4);

        header_offsets
    }
}

impl WriteToWriter for &[ModelHeader] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();

        self.iter().for_each(|header| {
            header.write_to_writer(writer);
        });

        // align data for vec type
        writer.align_size(4);

        offset
    }
}

impl WriteToWriter for ModelHeader {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();

        writer.append_u8_slice(self.name.as_slice());
        writer.append_i32(self.type_);
        writer.append_f32(self.bounding_radius);
        writer.append_i32(self.num_mesh); // num_mesh
        writer.append_i32(self.mesh_index); // mesh_index
        writer.append_i32(self.num_verts); // num_verts
        writer.append_i32(self.vert_info_index);
        writer.append_i32(self.vert_index);
        writer.append_i32(self.num_norms); // num_norms
        writer.append_i32(self.norm_info_index);
        writer.append_i32(self.norm_index);
        writer.append_i32(self.num_groups);
        writer.append_i32(self.group_index);

        assert_eq!(writer.get_offset() - offset, size_of::<ModelHeader>());

        // align data for vec type
        writer.align_size(4);

        offset
    }
}
