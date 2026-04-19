use std::io::{BufWriter, Write};

use crate::types::Smd;

macro_rules! write_dvec {
    ($buff:ident, $dvec:expr) => {{
        for e in $dvec.to_array() {
            $buff.write_all(format!("{} ", e).as_bytes())?;
        }
    }};
}

impl Smd {
    pub fn write_to_string(&self) -> eyre::Result<String> {
        let mut file = BufWriter::new(vec![]);

        file.write_all(format!("version {}\n", self.version).as_bytes())?;

        // nodes
        file.write_all("nodes\n".as_bytes())?;
        for node in &self.nodes {
            file.write_all(
                format!("{} \"{}\" {}\n", node.id, node.bone_name, node.parent).as_bytes(),
            )?
        }
        file.write_all("end\n".as_bytes())?;

        // skeleton
        file.write_all("skeleton\n".as_bytes())?;
        for skeleton in &self.skeleton {
            file.write_all(format!("time {}\n", skeleton.time).as_bytes())?;

            for bone in &skeleton.bones {
                file.write_all(format!("{} ", bone.id).as_bytes())?;
                write_dvec!(file, bone.pos);
                write_dvec!(file, bone.rot);
                file.write_all("\n".as_bytes())?;
            }
        }
        file.write_all("end\n".as_bytes())?;

        // triangles
        if !self.triangles.is_empty() {
            file.write_all("triangles\n".as_bytes())?;

            for triangle in &self.triangles {
                file.write_all(format!("{}\n", triangle.material).as_bytes())?;

                for vertex in &triangle.vertices {
                    file.write_all(format!("{} ", vertex.parent).as_bytes())?;
                    write_dvec!(file, vertex.pos);
                    write_dvec!(file, vertex.norm);
                    write_dvec!(file, vertex.uv);

                    if let Some(source) = &vertex.source {
                        file.write_all(format!("{}", source.links).as_bytes())?;

                        if let Some(bone) = source.bone {
                            file.write_all(format!(" {}", bone).as_bytes())?;
                        }

                        if let Some(weight) = source.weight {
                            file.write_all(format!(" {}", weight).as_bytes())?;
                        }
                    }

                    file.write_all("\n".as_bytes())?;
                }
            }
            file.write_all("end\n".as_bytes())?;
        }

        if !self.vertex_anim.is_empty() {
            // skeleton
            file.write_all("vertexanim\n".as_bytes())?;
            for single in &self.vertex_anim {
                file.write_all(format!("time {}\n", single.time).as_bytes())?;

                for vertex in &single.vertices {
                    file.write_all(format!("{} ", vertex.id).as_bytes())?;
                    write_dvec!(file, vertex.pos);
                    write_dvec!(file, vertex.norm);
                    file.write_all("\n".as_bytes())?;
                }
            }
            file.write_all("end\n".as_bytes())?;
        }

        file.flush()?;

        let out = file.into_inner()?;
        let out = String::from_utf8(out)?;

        Ok(out)
    }
}
