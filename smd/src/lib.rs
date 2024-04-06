use std::fs::OpenOptions;
use std::io::{self, BufWriter, Write};
use std::path::Path;

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
    pub source: Option<VertexSourceInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VertexSourceInfo {
    pub links: i32,
    pub bone: i32,
    pub weight: f64,
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
    pub triangles: Vec<Triangle>,
    pub vertex_anim: Option<Vec<VertexAnim>>,
}

macro_rules! write_dvec {
    ($buff:ident, $dvec:expr) => {{
        for e in $dvec.to_array() {
            $buff.write_all(format!("{} ", e).as_bytes())?;
        }
    }};
}

impl Smd {
    pub fn new(file_name: &str) -> Self {
        let path = Path::new(file_name);

        if let Ok(file) = std::fs::read_to_string(path) {
            match parse_smd(&file) {
                Ok((_, res)) => res,
                Err(err) => panic!("Cannot read file. {}", err),
            }
        } else {
            panic!("Cannot open file.")
        }
    }

    pub fn write(self, file_name: &str) -> io::Result<()> {
        let path = Path::new(file_name);

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let mut file = BufWriter::new(file);

        file.write_all(format!("version {}\n", self.version).as_bytes())?;

        // nodes
        file.write_all("nodes\n".as_bytes())?;
        for node in self.nodes {
            file.write_all(
                format!("{} \"{}\" {}\n", node.id, node.bone_name, node.parent).as_bytes(),
            )?
        }
        file.write_all("end\n".as_bytes())?;

        // skeleton
        file.write_all("skeleton\n".as_bytes())?;
        for skeleton in self.skeleton {
            file.write_all(format!("time {}\n", skeleton.time).as_bytes())?;

            for bone in skeleton.bones {
                file.write_all(format!("{} ", bone.id).as_bytes())?;
                write_dvec!(file, bone.pos);
                write_dvec!(file, bone.rot);
                file.write_all("\n".as_bytes())?;
            }
        }
        file.write_all("end\n".as_bytes())?;

        // triangles
        file.write_all("triangles\n".as_bytes())?;
        for triangle in self.triangles {
            file.write_all(format!("{}\n", triangle.material).as_bytes())?;

            for vertex in triangle.vertices {
                file.write_all(format!("{} ", vertex.parent).as_bytes())?;
                write_dvec!(file, vertex.pos);
                write_dvec!(file, vertex.norm);
                write_dvec!(file, vertex.uv);

                if let Some(source) = vertex.source {
                    file.write_all(
                        format!("{} {} {}", source.links, source.bone, source.weight).as_bytes(),
                    )?
                }

                file.write_all("\n".as_bytes())?;
            }
        }
        file.write_all("end\n".as_bytes())?;

        if let Some(vertex_anim) = self.vertex_anim {
            // skeleton
            file.write_all("vertexanim\n".as_bytes())?;
            for single in vertex_anim {
                file.write_all(format!("time {}\n", single.time).as_bytes())?;

                for vertex in single.vertices {
                    file.write_all(format!("{} ", vertex.id).as_bytes())?;
                    write_dvec!(file, vertex.pos);
                    write_dvec!(file, vertex.norm);
                    file.write_all("\n".as_bytes())?;
                }
            }
            file.write_all("end\n".as_bytes())?;
        }

        file.flush()?;

        Ok(())
    }
}

fn _number(i: &str) -> IResult<i32> {
    map_res(recognize(preceded(opt(tag("-")), digit1)), |s: &str| {
        s.parse::<i32>()
    })(i)
}

fn number(i: &str) -> IResult<i32> {
    preceded(space0, _number)(i)
}

fn signed_double(i: &str) -> IResult<f64> {
    map(recognize(preceded(opt(tag("-")), _double)), |what: &str| {
        what.parse().unwrap()
    })(i)
}

pub fn double(i: &str) -> IResult<f64> {
    preceded(space0, signed_double)(i)
}

fn quoted_text(i: &str) -> IResult<&str> {
    terminated(preceded(tag("\""), take_till(|c| c == '"')), tag("\""))(i)
}

fn in_block<'a, T>(
    s: &'a str,
    f: impl FnMut(&'a str) -> IResult<T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    terminated(
        preceded(tuple((space0, tag(s), multispace0)), f),
        tuple((space0, tag("end"), multispace0)),
    )
}

fn dvec3(i: &str) -> IResult<DVec3> {
    map(tuple((double, double, double)), |(x, y, z)| {
        DVec3::new(x, y, z)
    })(i)
}

fn dvec2(i: &str) -> IResult<DVec2> {
    map(tuple((double, double)), |(x, y)| DVec2::new(x, y))(i)
}

// Between space and end line.
fn between_space_and_endline<'a, T>(
    f: impl FnMut(&'a str) -> IResult<T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    terminated(preceded(multispace0, f), multispace0)
}

// Beware of the usage. If parsing to end of file,
// it should pair with many_till and eof
// many_till(take_line, eof)
fn take_line(i: &str) -> IResult<&str> {
    terminated(not_line_ending, multispace0)(i)
}

fn parse_header(i: &str) -> IResult<i32> {
    terminated(preceded(tag("version "), number), multispace0)(i)
}

// Main parsing functions
fn parse_node(i: &str) -> IResult<(i32, &str, i32)> {
    tuple((
        number,
        preceded(space1, quoted_text),
        preceded(space1, number),
    ))(i)
}

fn parse_nodes(i: &str) -> IResult<Vec<Node>> {
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

fn parse_bone_pos(i: &str) -> IResult<BonePos> {
    map(tuple((number, dvec3, dvec3)), |(id, pos, rot)| BonePos {
        id,
        pos,
        rot,
    })(i)
}

fn parse_bones(i: &str) -> IResult<Vec<BonePos>> {
    many0(between_space_and_endline(parse_bone_pos))(i)
}

fn parse_bones_single_time_frame(i: &str) -> IResult<Skeleton> {
    map(
        tuple((
            preceded(tag("time"), between_space_and_endline(number)),
            parse_bones,
        )),
        |(time, bones)| Skeleton { time, bones },
    )(i)
}

fn parse_skeleton(i: &str) -> IResult<Vec<Skeleton>> {
    in_block("skeleton", many0(parse_bones_single_time_frame))(i)
}

fn parse_vertex_source_info(i: &str) -> IResult<VertexSourceInfo> {
    map(tuple((number, number, double)), |(links, bone, weight)| {
        VertexSourceInfo {
            links,
            bone,
            weight,
        }
    })(i)
}

fn parse_vertex(i: &str) -> IResult<Vertex> {
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

fn parse_vertices(i: &str) -> IResult<Vec<Vertex>> {
    many0(between_space_and_endline(parse_vertex))(i)
}

fn parse_triangle(i: &str) -> IResult<Triangle> {
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

fn parse_triangles(i: &str) -> IResult<Vec<Triangle>> {
    in_block("triangles", many0(parse_triangle))(i)
}

fn parse_vertex_anim_pos(i: &str) -> IResult<VertexAnimPos> {
    map(tuple((number, dvec3, dvec3)), |(id, pos, norm)| {
        VertexAnimPos { id, pos, norm }
    })(i)
}

fn parse_vertex_anim_vertices(i: &str) -> IResult<Vec<VertexAnimPos>> {
    many0(between_space_and_endline(parse_vertex_anim_pos))(i)
}

fn parse_vertex_anim_single_time_frame(i: &str) -> IResult<VertexAnim> {
    map(
        tuple((
            preceded(tag("time"), between_space_and_endline(number)),
            parse_vertex_anim_vertices,
        )),
        |(time, vertices)| VertexAnim { time, vertices },
    )(i)
}

fn parse_vertex_anims(i: &str) -> IResult<Vec<VertexAnim>> {
    in_block(
        "vertexanimation",
        many0(parse_vertex_anim_single_time_frame),
    )(i)
}

fn parse_smd(i: &str) -> IResult<Smd> {
    map(
        tuple((
            parse_header,
            parse_nodes,
            parse_skeleton,
            parse_triangles,
            opt(parse_vertex_anims),
        )),
        |(version, nodes, skeleton, triangles, vertex_anim)| Smd {
            version,
            nodes,
            skeleton,
            triangles,
            vertex_anim,
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
        assert_eq!(tri1.vertices[1].source.as_ref().unwrap().bone, 1);
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
        Smd::new("./test/s1_r05_ref.smd");
    }

    #[test]
    fn goldsrc_file_read() {
        Smd::new("./test/cyberwave_goldsrc.smd");
    }

    #[test]
    fn goldsrc_file_read_write() {
        let file = Smd::new("./test/cyberwave_goldsrc.smd");

        file.write("./test/out/cyberwave_goldsrc_read_write.smd")
            .unwrap();
    }

    #[test]
    fn source_file_read_write() {
        let file = Smd::new("./test/s1_r05_ref.smd");

        file.write("./test/out/s1_r05_ref_read_write.smd").unwrap();
    }

    #[test]
    fn goldsrc_file_read_write_read() {
        let file = Smd::new("./test/cyberwave_goldsrc.smd");
        let file2 = Smd::new("./test/out/cyberwave_goldsrc_read_write.smd");

        assert_eq!(file, file2);
    }

    #[test]
    fn source_file_read_write_read() {
        let file = Smd::new("./test/s1_r05_ref.smd");
        let file2 = Smd::new("./test/out/s1_r05_ref_read_write.smd");

        assert_eq!(file, file2);
    }
}
