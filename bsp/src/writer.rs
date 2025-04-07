use std::{
    ffi::OsStr,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use byte_writer::ByteWriter;

use crate::{
    constants::{
        HEADER_LUMPS, HEADER_LUMP_SIZE, LUMP_CLIPNODES, LUMP_EDGES, LUMP_ENTITIES, LUMP_FACES,
        LUMP_LEAVES, LUMP_LIGHTING, LUMP_MARKSURFACES, LUMP_MODELS, LUMP_NODES, LUMP_PLANES,
        LUMP_SURFEDGES, LUMP_TEXINFO, LUMP_TEXTURES, LUMP_VERTICES, LUMP_VISIBILITY,
    },
    error::BspError,
    parse_bsp, Bsp, ClipNode, Face, Leaf, Model, TexInfo,
};

impl Bsp {
    pub fn from_bytes(bytes: &[u8]) -> Result<Bsp, BspError> {
        parse_bsp(bytes)
    }

    pub fn from_file(path: impl AsRef<Path> + AsRef<OsStr>) -> Result<Bsp, BspError> {
        let path: &Path = path.as_ref();

        let bytes = std::fs::read(path).map_err(|op| BspError::IOError {
            source: op,
            path: path.to_path_buf(),
        })?;
        Self::from_bytes(&bytes)
    }

    pub fn write_to_file(&self, path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<()> {
        let bytes = self.write_to_bytes();

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        file.write_all(&bytes)?;

        file.flush()?;

        Ok(())
    }

    pub fn write_to_bytes(&self) -> Vec<u8> {
        // for most compilers, there's 7 bytes of trailing null at the end of the file
        // not sure which lump it belongs to but entity lump is at the end if that clears up anything

        let mut writer = ByteWriter::new();

        writer.append_i32(30); // version

        // will be writing the offset later on
        let lump_headers_offset = writer.get_offset();
        let lump_headers_padding = vec![0u8; HEADER_LUMP_SIZE * HEADER_LUMPS];
        writer.append_u8_slice(&lump_headers_padding);

        // just writes all the lumps like normal then we go back to the lump header again
        // this means if we have weird lump header order, this could be changed easily
        // by changing the numbers in constants.rs

        // write entities
        {
            let offset = writer.get_offset();
            let mut entity_str = String::new();

            self.entities.iter().for_each(|entity| {
                // start with "{" then "\n"
                // "\n" at the end of key-value pair
                // " " to separate between key and value
                // ends "}" and no need for "\n" because it is from previous pair
                entity_str += "{\n";

                entity.iter().for_each(|(key, value)| {
                    entity_str += format!("\"{}\" \"{}\"\n", key, value).as_str()
                });

                // "\n" will separate entity
                entity_str += "}\n";
            });

            // null at the end for some reasons
            writer.append_string(entity_str.as_str());
            writer.append_u8(0);

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_ENTITIES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write planes
        {
            let offset = writer.get_offset();

            self.planes.iter().for_each(|plane| {
                writer.append_f32(plane.normal.x);
                writer.append_f32(plane.normal.y);
                writer.append_f32(plane.normal.z);

                writer.append_f32(plane.distance);
                writer.append_i32(plane.type_ as i32);
            });

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_PLANES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write textures
        {
            let offset = writer.get_offset();

            // texture count
            writer.append_u32(self.textures.len() as u32);

            // pad offset
            let offsets_start = writer.get_offset();
            (0..self.textures.len()).for_each(|_| {
                writer.append_i32(0); // dummy
            });

            self.textures.iter().enumerate().for_each(|(idx, texture)| {
                let texture_offset = writer.get_offset();

                // texture offset is relative to where the lump starts
                // for embedded texture, this is still needed
                writer.replace_with_u32(offsets_start + idx * 4, (texture_offset - offset) as u32);

                texture.write(&mut writer);
            });

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_TEXTURES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write vertices
        {
            let offset = writer.get_offset();

            self.vertices.iter().for_each(|vertex| {
                writer.append_f32(vertex.x);
                writer.append_f32(vertex.y);
                writer.append_f32(vertex.z);
            });

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_VERTICES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write visibility
        {
            let offset = writer.get_offset();

            // TODO
            writer.append_u8_slice(&self.visibility);

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_VISIBILITY * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write nodes
        {
            let offset = writer.get_offset();

            self.nodes.iter().for_each(|node| {
                writer.append_u32(node.plane);
                writer.append_i16(node.children[0]);
                writer.append_i16(node.children[1]);

                node.mins.iter().for_each(|&x| {
                    writer.append_i16(x);
                });
                node.maxs.iter().for_each(|&x| {
                    writer.append_i16(x);
                });

                writer.append_u16(node.first_face);
                writer.append_u16(node.face_count);
            });

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_NODES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write texinfo
        {
            let offset = writer.get_offset();

            self.texinfo.iter().for_each(
                |TexInfo {
                     u,
                     u_offset,
                     v,
                     v_offset,
                     texture_index,
                     flags,
                 }| {
                    writer.append_f32(u.x);
                    writer.append_f32(u.y);
                    writer.append_f32(u.z);
                    writer.append_f32(*u_offset);

                    writer.append_f32(v.x);
                    writer.append_f32(v.y);
                    writer.append_f32(v.z);
                    writer.append_f32(*v_offset);

                    writer.append_u32(*texture_index);
                    writer.append_u32(*flags);
                },
            );

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_TEXINFO * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write faces
        {
            let offset = writer.get_offset();

            self.faces.iter().for_each(
                |Face {
                     plane,
                     side,
                     first_edge,
                     edge_count,
                     texinfo,
                     styles,
                     lightmap_offset,
                 }| {
                    writer.append_u16(*plane);
                    writer.append_u16(*side);
                    writer.append_i32(*first_edge);
                    writer.append_u16(*edge_count);
                    writer.append_u16(*texinfo);

                    styles.iter().for_each(|&v| {
                        writer.append_u8(v);
                    });

                    writer.append_i32(*lightmap_offset);
                },
            );

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_FACES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write lightmap
        {
            let offset = writer.get_offset();

            self.lightmap.iter().for_each(|lightmap| {
                writer.append_u8_slice(lightmap);
            });

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_LIGHTING * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write clipnodes
        {
            let offset = writer.get_offset();

            self.clipnodes
                .iter()
                .for_each(|ClipNode { plane, children }| {
                    writer.append_i32(*plane);
                    writer.append_i16(children[0]);
                    writer.append_i16(children[1]);
                });

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_CLIPNODES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write leaves
        {
            let offset = writer.get_offset();

            self.leaves.iter().for_each(
                |Leaf {
                     contents,
                     vis_offset,
                     mins,
                     maxs,
                     first_mark_surface,
                     mark_surface_count,
                     ambient_levels,
                 }| {
                    writer.append_i32(*contents as i32);
                    writer.append_i32(*vis_offset);

                    mins.iter().for_each(|&v| {
                        writer.append_i16(v);
                    });
                    maxs.iter().for_each(|&v| {
                        writer.append_i16(v);
                    });

                    writer.append_u16(*first_mark_surface);
                    writer.append_u16(*mark_surface_count);

                    ambient_levels.iter().for_each(|&v| {
                        writer.append_u8(v);
                    });
                },
            );

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_LEAVES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write mark surfaces
        {
            let offset = writer.get_offset();

            self.mark_surfaces.iter().for_each(|&v| {
                writer.append_u16(v);
            });

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_MARKSURFACES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write edges
        {
            let offset = writer.get_offset();

            self.edges.iter().for_each(|&[p1, p2]| {
                writer.append_u16(p1);
                writer.append_u16(p2);
            });

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_EDGES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write surf edges
        {
            let offset = writer.get_offset();

            self.surf_edges.iter().for_each(|&v| {
                writer.append_i32(v);
            });

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_SURFEDGES * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        // write models
        {
            let offset = writer.get_offset();

            self.models.iter().for_each(
                |Model {
                     mins,
                     maxs,
                     origin,
                     head_nodes,
                     vis_leaves_count,
                     first_face,
                     face_count,
                 }| {
                    writer.append_f32(mins.x);
                    writer.append_f32(mins.y);
                    writer.append_f32(mins.z);
                    writer.append_f32(maxs.x);
                    writer.append_f32(maxs.y);
                    writer.append_f32(maxs.z);
                    writer.append_f32(origin.x);
                    writer.append_f32(origin.y);
                    writer.append_f32(origin.z);

                    head_nodes.iter().for_each(|&v| {
                        writer.append_i32(v);
                    });

                    writer.append_i32(*vis_leaves_count);
                    writer.append_i32(*first_face);
                    writer.append_i32(*face_count);
                },
            );

            let length = writer.get_offset() - offset;
            let header = lump_headers_offset + LUMP_MODELS * HEADER_LUMP_SIZE;

            writer.replace_with_i32(header, offset as i32);
            writer.replace_with_i32(header + 4, length as i32);
        }

        writer.data
    }
}
