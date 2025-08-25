use byte_writer::ByteWriter;

use crate::{writer::WriteToWriter, Bodypart, BodypartHeader, MeshHeader, Model, ModelHeader};

impl WriteToWriter for &[Bodypart] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        // write all models first
        let model_offsets: Vec<usize> = self
            .iter()
            .map(|bodypart| bodypart.models.as_slice().write_to_writer(writer))
            .collect();

        // write all headers then
        let header_offset = writer.get_offset();

        self.iter()
            .zip(model_offsets)
            .for_each(|(bodypart, model_offset)| {
                let BodypartHeader {
                    name,
                    num_models,
                    base,
                    model_index: _,
                } = bodypart.header;

                writer.append_u8_slice(name.as_slice());
                writer.append_i32(num_models);
                // should always be 1, please
                writer.append_i32(base);
                writer.append_i32(model_offset as i32);
            });

        header_offset
    }
}

impl WriteToWriter for &[Model] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        // write data first and then header
        // this is very tricky because each model has mesh, and each mesh has header

        // write all vertex and normal info
        let mut vert_info_writer = ByteWriter::new();
        let vert_info_offsets: Vec<usize> = self
            .iter()
            .map(|model| {
                let offset = writer.get_offset();

                vert_info_writer.append_u8_slice(&model.vertex_info);

                offset
            })
            .collect();

        let mut norm_info_writer = ByteWriter::new();
        let norm_info_offsets: Vec<usize> = self
            .iter()
            .map(|model| {
                let offset = writer.get_offset();

                norm_info_writer.append_u8_slice(&model.normal_info);

                offset
            })
            .collect();

        // write all vertices and normals
        let mut vert_writer = ByteWriter::new();
        let mut norm_writer = ByteWriter::new();

        let trivert_offsets: Vec<usize> = self
            .iter()
            .map(|model| {
                let offset = writer.get_offset();

                model.meshes.iter().for_each(|mesh| {
                    mesh.triangles.iter().for_each(|mesh_triangles| {
                        mesh_triangles.get_triverts().iter().for_each(|trivert| {
                            vert_writer.append_f32_slice(trivert.vertex.to_array().as_slice());
                            norm_writer.append_f32_slice(trivert.normal.to_array().as_slice());
                        });
                    });
                });

                offset
            })
            .collect();

        // appending the data to current writer and then we write mesh first and then we go back to model
        let vert_info_chunk_offset = writer.get_offset();
        writer.append_u8_slice(&vert_info_writer.data);

        let norm_info_chunk_offset = writer.get_offset();
        writer.append_u8_slice(&norm_info_writer.data);

        let vert_chunk_offset = writer.get_offset();
        writer.append_u8_slice(&vert_writer.data);

        let norm_chunk_offset = writer.get_offset();
        writer.append_u8_slice(&norm_writer.data);

        // now we write all trivert headers
        // write it directly to the main writer
        let trivert_header_offsets: Vec<Vec<usize>> = self
            .iter()
            .map(|model| {
                // we are allowed to have 64k vertices per model, very sad
                // this is because the way we get the vertex is by getting the vert_index/offset from the trivert
                // and then offset the "model_header.vert_index"
                // model vertex count is limited by vert_offset type, and it is i16
                let mut model_vertex_count = 0;

                model
                    .meshes
                    .iter()
                    .map(|mesh| {
                        let trivert_header_index = writer.get_offset();

                        // "mesh_triangles" here means a run of strip/fan.
                        mesh.triangles.iter().for_each(|mesh_triangles| {
                            // count
                            // must write with negative sign
                            writer.append_i16(mesh_triangles.len_and_type());

                            // header data
                            // just add as it goes because in the vertex block, we write it incrementally
                            mesh_triangles.get_triverts().iter().for_each(|trivert| {
                                // this is a choice
                                // trivert_header uses a mesh's data (tri_index) to know where the header run starts
                                // this means, the offset for vert and norm could be contained for that one run only
                                // but here, the vert and norm indices are using a variable (that counts all vertices in a model)
                                writer.append_i16(model_vertex_count);
                                writer.append_i16(model_vertex_count);
                                writer.append_i16(trivert.header.s);
                                writer.append_i16(trivert.header.t);

                                model_vertex_count += 1;
                            });
                        });

                        // need to write length 0 to stop triangle run
                        writer.append_i16(0);

                        trivert_header_index
                    })
                    .collect()
            })
            .collect();

        // write mesh headers
        let mesh_header_offsets: Vec<usize> = self
            .iter()
            .zip(trivert_header_offsets)
            .map(|(model, trivert_header_offset)| {
                let offset = writer.get_offset();

                model.meshes.iter().zip(trivert_header_offset).for_each(
                    |(mesh, trivert_header_index)| {
                        let MeshHeader {
                            num_tris: _,
                            tri_index: _,
                            skin_ref,
                            num_norms: _,
                            norm_index: _,
                        } = mesh.header;

                        let triangle_count: usize = mesh
                            .triangles
                            .iter()
                            .map(|x| x.len_and_type().abs() as usize)
                            .sum();

                        writer.append_i32(triangle_count as i32);

                        // next is tri_index
                        // it points to the the trivert run
                        writer.append_i32(trivert_header_index as i32);

                        writer.append_i32(skin_ref);

                        // norm, unused
                        writer.append_i32(triangle_count as i32);
                        writer.append_i32(norm_chunk_offset as i32); // idk
                    },
                );

                offset
            })
            .collect();

        // write model headers
        let model_headers_offset = writer.get_offset();

        self.iter().for_each(|model| {
            let ModelHeader {
                name,
                type_,
                bounding_radius,
                num_mesh: _,
                mesh_index: _,
                num_verts: _,
                vert_info_index: _,
                vert_index: _,
                num_norms: _,
                norm_info_index: _,
                norm_index: _,
                num_groups,
                group_index,
            } = model.header;

            writer.append_u8_slice(name.as_slice());
            writer.append_i32(type_);
            writer.append_f32(bounding_radius);
            writer.append_i32(model.meshes.len() as i32);
            writer.append_i32(mesh_header_offsets.get(0).cloned().unwrap_or(0) as i32); // mesh index, we only care about the first one
            writer.append_i32(model.vertex_info.len() as i32); // num_verts
            writer.append_i32(vert_info_chunk_offset as i32);
            writer.append_i32(vert_chunk_offset as i32);
            writer.append_i32(model.normal_info.len() as i32); // num_norms
            writer.append_i32(norm_info_chunk_offset as i32);
            writer.append_i32(norm_chunk_offset as i32);

            // unused fields
            writer.append_i32(num_groups);
            writer.append_i32(group_index);
        });

        model_headers_offset
    }
}
