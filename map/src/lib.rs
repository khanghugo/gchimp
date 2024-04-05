use std::{collections::HashMap, path::Path};

use glam::{DVec3, DVec4};
use nom::{
    bytes::complete::{tag, take_till},
    character::complete::{multispace0, space0},
    combinator::{all_consuming, map, opt, recognize},
    multi::{self, fold_many1, many0, many1},
    number::complete::double as _double,
    sequence::{delimited, preceded, terminated, tuple},
    IResult as _IResult,
};

#[derive(Debug)]
struct BrushPlane {
    p1: DVec3,
    p2: DVec3,
    p3: DVec3,
    texture_name: String,
    /// Ux Uy Uz Uoffset
    u: DVec4,
    /// Vx Vy Vz Voffset
    v: DVec4,
    rotation: f64,
    u_scale: f64,
    v_scale: f64,
}

#[derive(Debug)]
struct Brush {
    planes: Vec<BrushPlane>,
}

// #[derive(Debug)]
type Attributes = HashMap<String, String>;

#[derive(Debug)]
struct Entity {
    // All entities have attributes.
    attributes: Attributes,
    brushes: Option<Vec<Brush>>,
}

#[derive(Debug)]
pub struct Map {
    entities: Vec<Entity>,
}

impl Map {
    pub fn new(map_file: &str) -> Self {
        let path = Path::new(map_file);

        if let Ok(file) = std::fs::read_to_string(&path) {
            match parse_map(&file) {
                Ok((_, res)) => res,
                Err(err) => panic!("Cannot read file. {}", err),
            }
        } else {
            panic!("Cannot open file.")
        }
    }
}

type IResult<'a, T> = _IResult<&'a str, T>;

// TODO: make it not discard
// Many 0 because it doesn't necessary have it every time.
fn discard_comment_lines(i: &str) -> IResult<&str> {
    let discard_comment_line = terminated(
        preceded(tuple((space0, tag("//"))), take_till(|c| c == '\n')),
        multispace0,
    );

    map(many0(discard_comment_line), |_| "")(i)
}

fn signed_double(i: &str) -> IResult<f64> {
    map(recognize(preceded(opt(tag("-")), _double)), |what: &str| {
        what.parse().unwrap()
    })(i)
}

pub fn double(i: &str) -> IResult<f64> {
    preceded(space0, signed_double)(i)
}

fn between_line_bracket<'a, T>(
    f: impl FnMut(&'a str) -> IResult<T>,
) -> impl FnMut(&'a str) -> IResult<'a, T> {
    terminated(
        preceded(tuple((space0, tag("{"), multispace0)), f),
        tuple((space0, tag("}"), multispace0)),
    )
}

fn quoted_text(i: &str) -> IResult<&str> {
    terminated(preceded(tag("\""), take_till(|c| c == '"')), tag("\""))(i)
}

// For brushes
// These ones take in space0 at the end
// just to make sure that the next thing we read is a value.
fn parse_plane_coordinate(i: &str) -> IResult<DVec3> {
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

fn parse_plane_uv(i: &str) -> IResult<DVec4> {
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

fn parse_brush_plane(i: &str) -> IResult<BrushPlane> {
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
            texture_name,
            u,
            v,
            rotation,
            u_scale,
            v_scale,
        },
    )(i)
}

fn parse_brushes(i: &str) -> IResult<Vec<Brush>> {
    let parse_brush = move |i| {
        map(
            many1(terminated(parse_brush_plane, multispace0)),
            |planes| Brush { planes },
        )(i)
    };

    many1(delimited(
        discard_comment_lines,
        between_line_bracket(parse_brush),
        discard_comment_lines,
    ))(i)
}

// For attributes
fn parse_attributes(i: &str) -> IResult<Attributes> {
    let parse_attribute = move |i| tuple((quoted_text, preceded(space0, quoted_text)))(i);

    fold_many1(
        terminated(parse_attribute, multispace0),
        || Attributes::new(),
        |mut acc: Attributes, (key, value)| {
            acc.insert(key.to_owned(), value.to_owned());
            acc
        },
    )(i)
}

// For map
fn parse_entities(i: &str) -> IResult<Vec<Entity>> {
    let parse_entity = move |i| {
        map(
            tuple((parse_attributes, opt(parse_brushes))),
            |(attributes, brushes)| Entity {
                attributes,
                brushes,
            },
        )(i)
    };

    many1(delimited(
        discard_comment_lines,
        between_line_bracket(parse_entity),
        discard_comment_lines,
    ))(i)
}

fn parse_map(i: &str) -> IResult<Map> {
    map(all_consuming(parse_entities), |entities| Map { entities })(i)
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
        assert_eq!(a[0].planes[0].texture_name, "NULL");
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

        let (rest, a) = discard_comment_lines(i).unwrap();
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
        assert_eq!(ent.attributes.get("_tb_mod").unwrap(), "cstrike;cstrike_downloads");

        assert!(ent.brushes.is_some());

        let brushes = ent.brushes.as_ref().unwrap();

        assert_eq!(brushes.len(), 1);

        let brush = &brushes[0];

        assert_eq!(brush.planes[3].p2, DVec3::new(64., 65., 192.));
        assert_eq!(brush.planes[3].texture_name, "__TB_empty");
        assert_eq!(brush.planes[3].u.x, 1.);
    }
}
