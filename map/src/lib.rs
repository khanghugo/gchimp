use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use glam::{DVec3, DVec4};
use nom::{
    bytes::complete::{tag, take_till},
    character::complete::{multispace0, space0},
    combinator::{all_consuming, map, opt, recognize},
    multi::{fold_many1, many0, many1, many_m_n},
    number::complete::double as _double,
    sequence::{delimited, preceded, terminated, tuple},
    IResult as _IResult,
};

use eyre::eyre;

#[derive(Debug, Clone, PartialEq)]
pub struct BrushPlane {
    pub p1: DVec3,
    pub p2: DVec3,
    pub p3: DVec3,
    pub texture_name: TextureName,
    /// Ux Uy Uz Uoffset
    pub u: DVec4,
    /// Vx Vy Vz Voffset
    pub v: DVec4,
    pub rotation: f64,
    pub u_scale: f64,
    pub v_scale: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureName(String);

impl TextureName {
    pub fn new(s: String) -> Self {
        Self(s)
    }
    /// Texture name is uppercase
    ///
    /// Should use this method for doing comparison
    pub fn get_string_standard(&self) -> String {
        self.0.to_uppercase()
    }

    /// Simply returns the string
    pub fn get_string(&self) -> String {
        self.0.clone()
    }
}

impl TryFrom<&str> for BrushPlane {
    type Error = &'static str;

    fn try_from(value: &'_ str) -> Result<Self, Self::Error> {
        match parse_brush_plane(value) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(err.to_string().leak()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Brush {
    pub planes: Vec<BrushPlane>,
}

impl TryFrom<&str> for Brush {
    type Error = &'static str;

    fn try_from(value: &'_ str) -> Result<Self, Self::Error> {
        match parse_brush(value) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(err.to_string().leak()),
        }
    }
}

// #[derive(Debug, Clone, PartialEq)]
pub type Attributes = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    // All entities have attributes.
    pub attributes: Attributes,
    pub brushes: Option<Vec<Brush>>,
}

impl TryFrom<&str> for Entity {
    type Error = &'static str;

    fn try_from(value: &'_ str) -> Result<Self, Self::Error> {
        match parse_entity(value) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(err.to_string().leak()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Map {
    pub tb_header: Option<Vec<String>>,
    pub entities: Vec<Entity>,
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl Map {
    pub fn new() -> Self {
        Self {
            tb_header: None,
            entities: vec![],
        }
    }

    pub fn from_text(text: &'_ str) -> eyre::Result<Self> {
        match parse_map(text) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(eyre!("Cannot parse text: {}", err.to_string())),
        }
    }

    pub fn from_file(path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<Self> {
        let text = std::fs::read_to_string(path)?;

        Self::from_text(&text)
    }

    pub fn parse_entities(text: &str) -> eyre::Result<Vec<Entity>> {
        match parse_entities(text) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(eyre!("Cannot parse text: {}", err.to_string())),
        }
    }

    pub fn write(&self, path: impl AsRef<Path> + Into<PathBuf>) -> io::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let mut file = BufWriter::new(file);

        if let Some(tb_header) = &self.tb_header {
            for s in tb_header {
                file.write_all("//".as_bytes())?;
                file.write_all(s.as_bytes())?;
                file.write_all("\n".as_bytes())?;
            }
        }

        for (entity_index, entities) in self.entities.iter().enumerate() {
            file.write_all(format!("// entity {}\n", entity_index).as_bytes())?;

            file.write_all("{\n".as_bytes())?;

            for (key, value) in &entities.attributes {
                file.write_all(format!("\"{}\" \"{}\"\n", key, value).as_bytes())?;
            }

            if let Some(brushes) = &entities.brushes {
                for (brush_entity, brush) in brushes.iter().enumerate() {
                    file.write_all(format!("// brush {}\n", brush_entity).as_bytes())?;
                    file.write_all("{\n".as_bytes())?;

                    for plane in &brush.planes {
                        file.write_all(format!("( {} {} {} ) ( {} {} {} ) ( {} {} {} ) {} [ {} {} {} {} ] [ {} {} {} {} ] {} {} {}\n", 
                    plane.p1.x,plane.p1.y,plane.p1.z,
                    plane.p2.x,plane.p2.y,plane.p2.z,
                    plane.p3.x,plane.p3.y,plane.p3.z,
                    plane.texture_name.get_string(),
                    plane.u.x,plane.u.y,plane.u.z,plane.u.w,
                    plane.v.x,plane.v.y,plane.v.z,plane.v.w,
                    plane.rotation, plane.u_scale, plane.v_scale,

                ).as_bytes())?;
                    }
                    file.write_all("}\n".as_bytes())?;
                }
            }

            file.write_all("}\n".as_bytes())?;
        }

        file.flush()?;

        Ok(())
    }
}

type IResult<'a, T> = _IResult<&'a str, T>;

fn take_comment_line(i: &'_ str) -> IResult<'_, &str> {
    terminated(
        preceded(tuple((space0, tag("//"))), take_till(|c| c == '\n')),
        multispace0,
    )(i)
}

fn take_tb_header(i: &'_ str) -> IResult<'_, Vec<String>> {
    many_m_n(0, 2, map(take_comment_line, |i| i.to_string()))(i)
}

// TODO: make it not discard
// Many 0 because it doesn't necessary have it every time.
fn discard_comment_lines(i: &'_ str) -> IResult<'_, &str> {
    map(many0(take_comment_line), |_| "")(i)
}

fn signed_double(i: &'_ str) -> IResult<'_, f64> {
    map(recognize(preceded(opt(tag("-")), _double)), |what: &str| {
        what.parse().unwrap()
    })(i)
}

pub fn double(i: &'_ str) -> IResult<'_, f64> {
    preceded(space0, signed_double)(i)
}

fn between_line_bracket<'a, T>(
    f: impl FnMut(&'a str) -> IResult<'a, T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    terminated(
        preceded(tuple((space0, tag("{"), multispace0)), f),
        tuple((space0, tag("}"), multispace0)),
    )
}

fn quoted_text(i: &'_ str) -> IResult<'_, &str> {
    terminated(preceded(tag("\""), take_till(|c| c == '"')), tag("\""))(i)
}

// For brushes
// These ones take in space0 at the end
// just to make sure that the next thing we read is a value.
fn parse_plane_coordinate(i: &'_ str) -> IResult<'_, DVec3> {
    terminated(
        preceded(
            tuple((space0, tag("("), space0)),
            map(tuple((double, double, double)), |(x, y, z)| {
                DVec3::new(x, y, z)
            }),
        ),
        tuple((space0, tag(")"), space0)),
    )(i)
}

fn parse_plane_uv(i: &'_ str) -> IResult<'_, DVec4> {
    terminated(
        preceded(
            tuple((space0, tag("["), space0)),
            map(
                tuple((double, double, double, double)),
                |(x, y, z, offset)| DVec4::new(x, y, z, offset),
            ),
        ),
        tuple((space0, tag("]"), space0)),
    )(i)
}

fn parse_brush_plane(i: &'_ str) -> IResult<'_, BrushPlane> {
    map(
        tuple((
            parse_plane_coordinate,
            parse_plane_coordinate,
            parse_plane_coordinate,
            map(terminated(take_till(|c| c == ' '), space0), |s: &str| {
                s.to_string()
            }),
            parse_plane_uv,
            parse_plane_uv,
            double,
            double,
            double,
        )),
        |(p1, p2, p3, texture_name, u, v, rotation, u_scale, v_scale)| BrushPlane {
            p1,
            p2,
            p3,
            texture_name: TextureName(texture_name),
            u,
            v,
            rotation,
            u_scale,
            v_scale,
        },
    )(i)
}

fn parse_brush(i: &'_ str) -> IResult<'_, Brush> {
    map(
        many1(terminated(parse_brush_plane, multispace0)),
        |planes| Brush { planes },
    )(i)
}

fn parse_brushes(i: &'_ str) -> IResult<'_, Vec<Brush>> {
    many1(delimited(
        discard_comment_lines,
        between_line_bracket(parse_brush),
        discard_comment_lines,
    ))(i)
}

// For attributes
fn parse_attribute(i: &'_ str) -> IResult<'_, (&str, &'_ str)> {
    tuple((quoted_text, preceded(space0, quoted_text)))(i)
}

fn parse_attributes(i: &'_ str) -> IResult<'_, Attributes> {
    fold_many1(
        terminated(parse_attribute, multispace0),
        Attributes::new,
        |mut acc: Attributes, (key, value)| {
            acc.insert(key.to_owned(), value.to_owned());
            acc
        },
    )(i)
}

// For map
fn parse_entity(i: &'_ str) -> IResult<'_, Entity> {
    map(
        tuple((parse_attributes, opt(parse_brushes))),
        |(attributes, brushes)| Entity {
            attributes,
            brushes,
        },
    )(i)
}

fn parse_entities(i: &'_ str) -> IResult<'_, Vec<Entity>> {
    many1(delimited(
        discard_comment_lines,
        between_line_bracket(parse_entity),
        discard_comment_lines,
    ))(i)
}

fn parse_map(i: &'_ str) -> IResult<'_, Map> {
    map(
        all_consuming(tuple((opt(take_tb_header), parse_entities))),
        |(tb_header, entities)| Map {
            tb_header,
            entities,
        },
    )(i)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn inside_quote() {
        let i = "\"heyhey\"";

        let (rest, a) = quoted_text(i).unwrap();
        assert_eq!(a, "heyhey");
        assert_eq!(rest, "");
    }

    #[test]
    fn inside_bracket() {
        let i = "{
a
}
";

        let (rest, a) = between_line_bracket(terminated(tag("a"), multispace0))(i).unwrap();
        assert_eq!(a, "a");
        assert_eq!(rest, "");
    }

    #[test]
    fn brushes_parse() {
        let i = "\
// brush 4
{
( -120 -136 144 ) ( -120 -136 136 ) ( -120 56 144 ) NULL [ 2.220446049250313e-16 0 -1 24 ] [ 0 -1 0 0 ] 0 1 1
( 56 -136 144 ) ( 56 -136 136 ) ( -120 -136 144 ) NULL [ 1 0 0 0 ] [ 0 -2.220446049250313e-16 1 -8 ] 0 1 1
( 56 56 136 ) ( -120 56 136 ) ( 56 -136 136 ) sky [ 0 -1 0 0 ] [ -1 0 -2.220446049250313e-16 -256 ] 0 1 1
( 56 56 144 ) ( 56 -136 144 ) ( -120 56 144 ) NULL [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( -120 56 144 ) ( -120 56 136 ) ( 56 56 144 ) NULL [ 1 0 0 0 ] [ 0 -2.220446049250313e-16 -1 24 ] 0 1 1
( 56 56 144 ) ( 56 56 136 ) ( 56 -136 144 ) NULL [ 2.220446049250313e-16 0 1 -24 ] [ 0 -1 0 0 ] 0 1 1
}
// brush 5
{
( -120 56 -16 ) ( -120 56 -8 ) ( -120 -136 -16 ) NULL [ 2.220446049250313e-16 0 -1 24 ] [ 0 -1 0 0 ] 0 1 1
( -120 -136 -16 ) ( -120 -136 -8 ) ( 56 -136 -16 ) NULL [ 1 0 0 0 ] [ 0 -2.220446049250313e-16 1 -8 ] 0 1 1
( -120 56 -16 ) ( -120 -136 -16 ) ( 56 56 -16 ) NULL [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( -120 -136 -8 ) ( -120 56 -8 ) ( 56 -136 -8 ) tf [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 56 56 -16 ) ( 56 56 -8 ) ( -120 56 -16 ) NULL [ 1 0 0 0 ] [ 0 -2.220446049250313e-16 -1 24 ] 0 1 1
( 56 -136 -16 ) ( 56 -136 -8 ) ( 56 56 -16 ) NULL [ 2.220446049250313e-16 0 1 -24 ] [ 0 -1 0 0 ] 0 1 1
}
";

        let (_, a) = parse_brushes(i).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].planes[0].p1, DVec3::new(-120., -136., 144.));
        assert_eq!(a[0].planes[0].texture_name.get_string(), "NULL");
        assert_eq!(a[0].planes[0].u.x, 2.220446049250313e-16);
    }

    #[test]
    fn entities_parse() {
        let i = "\
{
\"classname\" \"info_player_start\"
\"spawnflags\" \"0\"
\"angles\" \"0 0 0\"
\"origin\" \"-80 -88 60\"
}";

        let (rest, a) = parse_entities(i).unwrap();
        assert_eq!(rest, "");
        assert_eq!(a.len(), 1);

        let ent = &a[0];

        assert!(ent.brushes.is_none());
        assert_eq!(ent.attributes.len(), 4);
        assert_eq!(ent.attributes.get("origin").unwrap(), "-80 -88 60");
    }

    #[test]
    fn comment_line_parse() {
        let i = "\
// A song for the broken heart
// Eh
// {} 
// \"\"";

        let (rest, _) = discard_comment_lines(i).unwrap();
        assert!(rest.is_empty());
    }

    #[test]
    fn file_parse() {
        let i = "\
// Game: Half-Life
// Format: Valve
// entity 0
{
\"mapversion\" \"220\"
\"wad\" \"/home/khang/map_compiler/sdhlt.wad;/home/khang/map_compiler/devtextures.wad\"
\"classname\" \"worldspawn\"
\"_tb_mod\" \"cstrike;cstrike_downloads\"
// brush 0
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) __TB_empty [ 0 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) __TB_empty [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) __TB_empty [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 64 64 192 ) ( 64 65 192 ) ( 65 64 192 ) __TB_empty [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) __TB_empty [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) __TB_empty [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}
}

";

        let (rest, a) = parse_map(i).unwrap();

        assert!(rest.is_empty());
        assert_eq!(a.entities.len(), 1);

        let ent = &a.entities[0];

        assert_eq!(ent.attributes.len(), 4);
        assert_eq!(
            ent.attributes.get("_tb_mod").unwrap(),
            "cstrike;cstrike_downloads"
        );

        assert!(ent.brushes.is_some());

        let brushes = ent.brushes.as_ref().unwrap();

        assert_eq!(brushes.len(), 1);

        let brush = &brushes[0];

        assert_eq!(brush.planes[3].p2, DVec3::new(64., 65., 192.));
        assert_eq!(brush.planes[3].texture_name.get_string(), "__TB_empty");
        assert_eq!(brush.planes[3].u.x, 1.);
    }

    #[test]
    fn file_read() {
        assert!(Map::from_file("./test/sky_vis.map").is_ok());
    }

    #[test]
    fn file_write_read() {
        let i = Map::from_file("./test/sky_vis.map").unwrap();
        i.write("./test/out/sky_vis_out.map").unwrap();

        let i = Map::from_file("./test/sky_vis.map").unwrap();
        let j = Map::from_file("./test/out/sky_vis_out.map").unwrap();

        assert_eq!(i, j);
    }

    #[test]
    fn fail_read() {
        let file = Map::from_file("./dunkin/do.nut");

        assert!(file.is_err());
    }

    #[test]
    fn read_copied_tb_entity() {
        let entity_text = "\
// entity 0
{
\"classname\" \"func_detail\"
\"zhlt_detaillevel\" \"1\"
\"zhlt_chopdown\" \"0\"
\"zhlt_chopup\" \"0\"
\"zhlt_coplanarpriority\" \"0\"
\"zhlt_clipnodedetaillevel\" \"1\"
// brush 0
{
( 6080 2256 -7552 ) ( 6080 2064 -7680 ) ( 6080 2064 -7808 ) measure128_01 [ 0 -1 0 108 ] [ 0 0 -1 0 ] 0 8 8
( 6080 2064 -7680 ) ( 6496 2064 -7680 ) ( 6496 2064 -7808 ) measure128_01 [ -1 0 0 -204 ] [ 0 0 -2 -208 ] 90 8 8
( 6496 2064 -7680 ) ( 6080 2064 -7680 ) ( 6080 2256 -7552 ) measure128_01 [ 1 0 0 204 ] [ 0 -0.692307692307634 -0.4615384615385487 -158.46155 ] 90 8 8
( 6496 2064 -7808 ) ( 6496 2256 -7808 ) ( 6080 2256 -7808 ) measure128_01 [ 1 0 0 204 ] [ 0 -1 0 108 ] 0 8 8
( 6496 2256 -7808 ) ( 6496 2256 -7552 ) ( 6080 2256 -7552 ) measure128_01 [ -1 0 0 -204 ] [ 0 0 -1 0 ] 0 8 8
( 6496 2064 -7680 ) ( 6496 2256 -7552 ) ( 6496 2256 -7808 ) NULL [ -2.220446049250313e-16 -1 0 0 ] [ 0 0 -1 0 ] 180 8 8
}
// brush 1
{
( 6080 2064 -7680 ) ( 6080 1872 -7552 ) ( 6080 1872 -7808 ) measure128_01 [ 0 -1 0 108 ] [ 0 0 -1 0 ] 0 8 8
( 6080 1872 -7552 ) ( 6496 1872 -7552 ) ( 6496 1872 -7808 ) measure128_01 [ -1 0 0 -204 ] [ 0 0 -1 0 ] 90 8 8
( 6496 1872 -7808 ) ( 6496 2064 -7808 ) ( 6080 2064 -7808 ) measure128_01 [ 1 0 0 204 ] [ 0 -1 0 108 ] 0 8 8
( 6080 2064 -7680 ) ( 6496 2064 -7680 ) ( 6496 1872 -7552 ) measure128_01 [ 1 0 0 204 ] [ 0 -0.6923076923077213 0.4615384615384178 215.69229 ] 90 8 8
( 6496 2064 -7808 ) ( 6496 2064 -7680 ) ( 6080 2064 -7680 ) measure128_01 [ -1 0 0 -204 ] [ 0 0 -2 -208 ] 270 8 8
( 6496 1872 -7552 ) ( 6496 2064 -7680 ) ( 6496 2064 -7808 ) NULL [ -2.220446049250313e-16 -1 0 0 ] [ 0 0 -1 0 ] 180 8 8
}
// brush 2
{
( 6080 2704 -6560 ) ( 6080 2688 -6560 ) ( 6080 2704 -8192 ) measure128_01 [ 0 -1 0 108 ] [ 0 0 -1 0 ] 0 8 8
( 6496 1872 -6560 ) ( 6496 1872 -8192 ) ( 5248 1872 -6560 ) measure128_01 [ -1 0 0 -204 ] [ 0 0 -1 0 ] 90 8 8
( 5248 2704 -8176 ) ( 5248 2688 -8176 ) ( 6496 2704 -8176 ) NULL [ -1 0 0 -12 ] [ 0 -1 0 12 ] 90 8 8
( 6496 2704 -7808 ) ( 6496 2688 -7808 ) ( 5248 2704 -7808 ) measure128_01 [ 1 0 0 204 ] [ 0 -1 0 108 ] 0 8 8
( 6496 2256 -6560 ) ( 5248 2256 -6560 ) ( 6496 2256 -8192 ) measure128_01 [ -1 0 0 -204 ] [ 0 0 -1 0 ] 0 8 8
( 6160 2704 -8192 ) ( 6160 2688 -8192 ) ( 6160 2704 -6560 ) measure128_01 [ -2.220446049250313e-16 -1 0 -166 ] [ 0 0 -1 0 ] 0 8 8
}
}
";

        let (_, _entities) = parse_map(entity_text).unwrap();
    }
}
