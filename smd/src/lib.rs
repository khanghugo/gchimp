use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use glam::{DVec2, DVec3};
use nom::bytes::complete::take_till;
use nom::character::complete::{digit1, multispace0, not_line_ending, space0, space1};
use nom::combinator::{map, map_res, not, opt, recognize};
use nom::multi::many0;
use nom::sequence::{terminated, tuple};
use nom::{
    bytes::complete::tag, number::complete::double as _double, sequence::preceded,
    IResult as _IResult,
};
type IResult<'a, T> = _IResult<&'a str, T>;

use eyre::eyre;

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

fn _number(i: &'_ str) -> IResult<'_, i32> {
    map_res(recognize(preceded(opt(tag("-")), digit1)), |s: &str| {
        s.parse::<i32>()
    })(i)
}

fn number(i: &'_ str) -> IResult<'_, i32> {
    preceded(space0, _number)(i)
}

fn signed_double(i: &'_ str) -> IResult<'_, f64> {
    map(recognize(preceded(opt(tag("-")), _double)), |what: &str| {
        what.parse().unwrap()
    })(i)
}

pub fn double(i: &'_ str) -> IResult<'_, f64> {
    preceded(space0, signed_double)(i)
}

fn quoted_text(i: &'_ str) -> IResult<'_, &str> {
    terminated(preceded(tag("\""), take_till(|c| c == '"')), tag("\""))(i)
}

fn in_block<'a, T>(
    s: &'a str,
    f: impl FnMut(&'a str) -> IResult<'a, T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    terminated(
        preceded(tuple((space0, tag(s), multispace0)), f),
        tuple((space0, tag("end"), multispace0)),
    )
}

fn dvec3(i: &'_ str) -> IResult<'_, DVec3> {
    map(tuple((double, double, double)), |(x, y, z)| {
        DVec3::new(x, y, z)
    })(i)
}

fn dvec2(i: &'_ str) -> IResult<'_, DVec2> {
    map(tuple((double, double)), |(x, y)| DVec2::new(x, y))(i)
}

// Between space and end line.
fn between_space_and_endline<'a, T>(
    f: impl FnMut(&'a str) -> IResult<'a, T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    terminated(preceded(multispace0, f), multispace0)
}

// Beware of the usage. If parsing to end of file,
// it should pair with many_till and eof
// many_till(take_line, eof)
fn take_line(i: &'_ str) -> IResult<'_, &str> {
    terminated(not_line_ending, multispace0)(i)
}

fn parse_header(i: &'_ str) -> IResult<'_, i32> {
    terminated(preceded(tag("version "), number), multispace0)(i)
}

// Main parsing functions
fn parse_node(i: &'_ str) -> IResult<'_, (i32, &str, i32)> {
    tuple((
        number,
        preceded(space1, quoted_text),
        preceded(space1, number),
    ))(i)
}

fn parse_nodes(i: &'_ str) -> IResult<'_, Vec<Node>> {
    in_block(
        "nodes",
        many0(map(
            between_space_and_endline(parse_node),
            |(id, bone_name, parent)| Node {
                id,
                bone_name: bone_name.to_owned(),
                parent,
            },
        )),
    )(i)
}

fn parse_bone_pos(i: &'_ str) -> IResult<'_, BonePos> {
    map(tuple((number, dvec3, dvec3)), |(id, pos, rot)| BonePos {
        id,
        pos,
        rot,
    })(i)
}

fn parse_bones(i: &'_ str) -> IResult<'_, Vec<BonePos>> {
    many0(between_space_and_endline(parse_bone_pos))(i)
}

fn parse_bones_single_time_frame(i: &'_ str) -> IResult<'_, Skeleton> {
    map(
        tuple((
            preceded(tag("time"), between_space_and_endline(number)),
            parse_bones,
        )),
        |(time, bones)| Skeleton { time, bones },
    )(i)
}

fn parse_skeleton(i: &'_ str) -> IResult<'_, Vec<Skeleton>> {
    in_block("skeleton", many0(parse_bones_single_time_frame))(i)
}

fn parse_vertex_source_info(i: &'_ str) -> IResult<'_, VertexSourceInfo> {
    map(
        tuple((number, opt(number), opt(double))),
        |(links, bone, weight)| VertexSourceInfo {
            links,
            bone,
            weight,
        },
    )(i)
}

fn parse_vertex(i: &'_ str) -> IResult<'_, Vertex> {
    map(
        tuple((number, dvec3, dvec3, dvec2, opt(parse_vertex_source_info))),
        |(parent, pos, norm, uv, source)| Vertex {
            parent,
            pos,
            norm,
            uv,
            source,
        },
    )(i)
}

fn parse_vertices(i: &'_ str) -> IResult<'_, Vec<Vertex>> {
    many0(between_space_and_endline(parse_vertex))(i)
}

fn parse_triangle(i: &'_ str) -> IResult<'_, Triangle> {
    // We cannot have a another line that is named "end" out of nowhere.
    // So no texture name "end" is allowed.
    map(
        tuple((not(tag("end")), take_line, parse_vertices)),
        |(_, material, vertices)| Triangle {
            material: material.to_string(),
            vertices,
        },
    )(i)
}

fn parse_triangles(i: &'_ str) -> IResult<'_, Vec<Triangle>> {
    in_block("triangles", many0(parse_triangle))(i)
}

fn parse_vertex_anim_pos(i: &'_ str) -> IResult<'_, VertexAnimPos> {
    map(tuple((number, dvec3, dvec3)), |(id, pos, norm)| {
        VertexAnimPos { id, pos, norm }
    })(i)
}

fn parse_vertex_anim_vertices(i: &'_ str) -> IResult<'_, Vec<VertexAnimPos>> {
    many0(between_space_and_endline(parse_vertex_anim_pos))(i)
}

fn parse_vertex_anim_single_time_frame(i: &'_ str) -> IResult<'_, VertexAnim> {
    map(
        tuple((
            preceded(tag("time"), between_space_and_endline(number)),
            parse_vertex_anim_vertices,
        )),
        |(time, vertices)| VertexAnim { time, vertices },
    )(i)
}

fn parse_vertex_anims(i: &'_ str) -> IResult<'_, Vec<VertexAnim>> {
    in_block(
        "vertexanimation",
        many0(parse_vertex_anim_single_time_frame),
    )(i)
}

fn discard_comment_line(i: &'_ str) -> IResult<'_, &str> {
    terminated(
        preceded(
            tuple((multispace0, tag("//"))),
            take_till(|c| c == '\n' || c == '\r'),
        ),
        multispace0,
    )(i)
}

fn parse_smd(i: &'_ str) -> IResult<'_, Smd> {
    map(
        tuple((
            opt(many0(discard_comment_line)),
            parse_header,
            parse_nodes,
            parse_skeleton,
            opt(parse_triangles),
            opt(parse_vertex_anims),
        )),
        |(_, version, nodes, skeleton, triangles, vertex_anim)| Smd {
            version,
            nodes,
            skeleton,
            triangles: triangles.unwrap_or(vec![]),
            vertex_anim: vertex_anim.unwrap_or(vec![]),
        },
    )(i)
}

#[cfg(test)]
mod test {
    use nom::{combinator::eof, multi::many_till};

    use super::*;

    #[test]
    fn space_and_endline() {
        let i = " aaa
";
        let (rest, a) = between_space_and_endline(tag("aaa"))(i).unwrap();
        assert!(rest.is_empty());
        assert_eq!(a, "aaa");
    }

    #[test]
    fn line() {
        let i = "\
aaa
bbb
ccc
";
        let (i, line1) = take_line(i).unwrap();
        let (i, line2) = take_line(i).unwrap();
        let (rest, line3) = take_line(i).unwrap();

        assert!(rest.is_empty());

        assert_eq!(line1, "aaa");
        assert_eq!(line2, "bbb");
        assert_eq!(line3, "ccc");
    }

    #[test]
    fn line2() {
        let i = "\
aaa
bbb
ccc

/
";
        let (rest, _) = many_till(take_line, eof)(i).unwrap();

        assert!(rest.is_empty());
    }

    #[test]
    fn header_parse() {
        let i = "version 19";
        let (_, version) = parse_header(i).unwrap();

        assert_eq!(version, 19);
    }

    #[test]
    fn node_parse() {
        let i = "0 \"root\" -1";
        let (_, node) = parse_node(i).unwrap();
        assert_eq!(node.1, "root");
        assert_eq!(node.0, 0);
        assert_eq!(node.2, -1);
    }

    #[test]
    fn nodes_parse() {
        let i = "\
nodes
0 \"root\" -1
1 \"child\" 0
end
";
        let (rest, nodes) = parse_nodes(i).unwrap();
        assert!(rest.is_empty());

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].bone_name, "root");
        assert_eq!(nodes[0].id, 0);
        assert_eq!(nodes[0].parent, -1);
        assert_eq!(nodes[1].bone_name, "child");
        assert_eq!(nodes[1].id, 1);
        assert_eq!(nodes[1].parent, 0);
    }

    #[test]
    fn skeleton_parse() {
        let i = "\
skeleton
time 0
0	0 0 0	1.570796 0 0
1	1 0 0	0 0 0
time 1
1	1 2 0	0 0 0
time 2
1	1 0 0	0 0 0
end
";

        let (rest, skeleton) = parse_skeleton(i).unwrap();
        assert!(rest.is_empty());

        assert_eq!(skeleton.len(), 3);

        let t1 = &skeleton[0];

        assert_eq!(t1.time, 0);
        assert_eq!(t1.bones.len(), 2);
        assert_eq!(t1.bones[0].rot.x, 1.570796);
        assert_eq!(t1.bones[1].rot.x, 0.);
    }

    #[test]
    fn vertex_parse() {
        let i = "0	0 0 0	0 0 1	0 1	1 0 1";

        let (rest, vertex) = parse_vertex(i).unwrap();
        assert!(rest.is_empty());

        assert_eq!(vertex.parent, 0);
        assert_eq!(vertex.norm.z, 1.);
        assert_eq!(vertex.uv.y, 1.);
        assert!(vertex.source.is_some());
        assert_eq!(vertex.source.as_ref().unwrap().links, 1);
    }

    #[test]
    fn vertices_parse() {
        let i = "\
0	0 0 0	0 0 1	0 1	1 0 1
0	0 0 0	0 0 1	0 1	1 0 1
";

        let (rest, vertices) = parse_vertices(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(vertices.len(), 2);

        let vertex = &vertices[1];

        assert_eq!(vertex.parent, 0);
        assert_eq!(vertex.norm.z, 1.);
        assert_eq!(vertex.uv.y, 1.);
        assert!(vertex.source.is_some());
        assert_eq!(vertex.source.as_ref().unwrap().links, 1);
    }

    #[test]
    fn triangle_parse() {
        let i = "\
my_material.bmp
0	0 0 0	0 0 1	0 1	1 0 1
0	0 -1 0	0 0 1	0 0	1 0 1
1	1 -1 0	0 0 1	1 0	1 1 1
";

        let (rest, triangle) = parse_triangle(i).unwrap();

        assert!(rest.is_empty());

        assert_eq!(triangle.material, "my_material.bmp");
    }

    #[test]
    fn triangles_source_parse() {
        let i = "\
triangles
my_material.bmp
0	0 0 0	0 0 1	0 1	1 0 1
0	0 -1 0	0 0 1	0 0	1 0 1
1	1 -1 0	0 0 1	1 0	1 1 1
my_material.bmp
0	0 0 0	1 0 1	0 1	1 0 1
1	1 -1 0	1 0 1	1 0	1 1 1
1	1 0 0	1 0 1	1 1	1 1 1
my_material.bmp
1	1 -1 0	0 0 1	1 0	1 1 1
0	0 -1 0	0 0 1	0 0	1 0 1
0	0 0 0	0 0 1	0 1	1 0 1
my_material.bmp
1	1 0 0	1 0 1	1 1	1 1 1
1	1 -1 0	1 0 1	1 0	1 1 1
0	0 0 0	1 0 1	0 1	1 0 1
end
";
        let (rest, triangles) = parse_triangles(i).unwrap();

        assert!(rest.is_empty());

        assert_eq!(triangles.len(), 4);

        let tri1 = &triangles[1];

        assert_eq!(tri1.material, "my_material.bmp");
        assert_eq!(tri1.vertices[1].parent, 1);
        assert_eq!(tri1.vertices[1].pos.y, -1.);

        assert!(tri1.vertices[1].source.is_some());
        assert_eq!(tri1.vertices[1].source.as_ref().unwrap().bone.unwrap(), 1);
    }

    // Won't test for vertexanimation. Too bad.
    #[test]
    fn smd_source_parse() {
        let i = "\
version 1
nodes
  0 \"static_prop\" -1
end
skeleton
  time 0
    0 0.000000 0.000000 0.000000 0.000000 0.000000 0.000000
end
triangles
metal_light_01_dark
  0 64.004799 -992.000000 272.000000 0.493988 0.000001 0.869469 -1.468750 5.000000 1 0 1.000000
  0 79.999947 1248.000000 251.991211 0.855284 0.000003 0.518159 0.718750 4.949997 1 0 1.000000
  0 64.004471 1248.000000 272.000000 0.370471 0.000000 0.928844 0.718750 5.000005 1 0 1.000000
end
";

        let (rest, smd) = parse_smd(i).unwrap();
        assert!(rest.is_empty());

        assert_eq!(smd.version, 1);
        assert_eq!(smd.nodes.len(), 1);
        assert_eq!(smd.skeleton.len(), 1);
        assert_eq!(smd.triangles.len(), 1);
    }

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

    #[test]
    fn parse_vertex_weird() {
        // in source info, only link is there
        let line =
            "0  -0.680206 -1.746510 5.699451  -0.977121 -0.159969 -0.140159  0.090947 1.911250 0";
        let (res, vertex) = parse_vertex(line).unwrap();

        assert!(res.is_empty());

        assert!(vertex.source.is_some());
        assert_eq!(vertex.source.unwrap().links, 0);
    }
}
