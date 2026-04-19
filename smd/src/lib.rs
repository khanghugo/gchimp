use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use glam::{DVec2, DVec3};

use eyre::eyre;

use crate::parser::parse_smd;
use crate::types::{BonePos, Node, Skeleton, Smd, Triangle};

pub mod parser;
pub mod types;
pub mod utils;

macro_rules! write_dvec {
    ($buff:ident, $dvec:expr) => {{
        for e in $dvec.to_array() {
            $buff.write_all(format!("{} ", e).as_bytes())?;
        }
    }};
}

impl Default for Smd {
    fn default() -> Self {
        Self::new()
    }
}

impl Smd {
    /// Creates a new [`Smd`] without any data
    pub fn new() -> Self {
        Self {
            version: 1,
            nodes: vec![],
            skeleton: vec![],
            triangles: vec![],
            vertex_anim: vec![],
        }
    }

    /// Creates a new [`Smd`] with the following data
    /// ```
    /// version 1
    /// nodes
    /// 0 "static_prop" -1
    /// end
    /// skeleton
    /// time 0
    ///   0 0.000000 0.000000 0.000000 0.000000 0.000000 0.000000
    /// end
    /// ```
    pub fn new_basic() -> Self {
        Self {
            version: 1,
            nodes: vec![Node {
                id: 0,
                bone_name: "static_prop".to_string(),
                parent: -1,
            }],
            skeleton: vec![Skeleton {
                time: 0,
                bones: vec![BonePos {
                    id: 0,
                    pos: [0., 0., 0.].into(),
                    rot: [0., 0., 0.].into(),
                }],
            }],
            triangles: vec![],
            vertex_anim: vec![],
        }
    }

    pub fn from(text: &'_ str) -> eyre::Result<Self> {
        match parse_smd(text) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(eyre!("Cannot parse text: {}", err.to_string())),
        }
    }

    pub fn from_file(path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<Self> {
        let text = std::fs::read_to_string(path)?;

        Self::from(&text)
    }

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

    pub fn write(&self, path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let mut file = BufWriter::new(file);

        let res_str = self.write_to_string()?;

        file.write_all(res_str.as_bytes())?;

        file.flush()?;

        Ok(())
    }

    pub fn add_triangle(&mut self, tri: Triangle) -> &mut Self {
        self.triangles.push(tri);

        self
    }

    pub fn without_triangles(&self) -> Self {
        Self {
            version: self.version,
            nodes: self.nodes.clone(),
            skeleton: self.skeleton.clone(),
            triangles: vec![],
            vertex_anim: self.vertex_anim.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{parser::parse_vertex, types::Smd};

    #[test]
    fn source_file_read() {
        assert!(Smd::from_file("./test/s1_r05_ref.smd").is_ok());
    }

    #[test]
    fn goldsrc_file_read() {
        assert!(Smd::from_file("./test/cyberwave_goldsrc.smd").is_ok());
    }

    #[test]
    fn goldsrc_file_read_write() {
        let file = Smd::from_file("./test/cyberwave_goldsrc.smd").unwrap();

        file.write("./test/out/cyberwave_goldsrc_read_write.smd")
            .unwrap();

        let file = Smd::from_file("./test/cyberwave_goldsrc.smd").unwrap();
        let file2 = Smd::from_file("./test/out/cyberwave_goldsrc_read_write.smd").unwrap();

        assert_eq!(file, file2);
    }

    #[test]
    fn source_file_read_write_read() {
        let file = Smd::from_file("./test/s1_r05_ref.smd").unwrap();

        file.write("./test/out/s1_r05_ref_read_write.smd").unwrap();

        let file = Smd::from_file("./test/s1_r05_ref.smd").unwrap();
        let file2 = Smd::from_file("./test/out/s1_r05_ref_read_write.smd").unwrap();

        assert_eq!(file, file2);
    }

    #[test]
    fn fail_read() {
        let file = Smd::from_file("./dunkin/do.nut");

        assert!(file.is_err());
    }

    #[test]
    fn parse_epiphany() {
        let file = Smd::from_file("test/willbreakanyway_001_ref.smd");

        assert!(file.is_ok());
    }

    #[test]
    fn parse_sequence_smd() {
        let file = Smd::from_file("test/idle.smd");

        assert!(file.is_ok());
    }
}
