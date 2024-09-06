use std::str::from_utf8;

use glam::Vec3;
use nom::{
    character::complete::multispace0,
    combinator::{all_consuming, fail, map, rest},
    error::context,
    multi::{count, many0},
    number::complete::{le_f32, le_i16, le_i32, le_u16, le_u32, le_u8},
    sequence::{delimited, tuple},
};
use wad::parse_miptex;

use crate::{
    constants::{
        BSP_VERSION, HEADER_LUMPS, LUMP_CLIPNODES, LUMP_EDGES, LUMP_ENTITIES, LUMP_FACES,
        LUMP_LEAVES, LUMP_LIGHTING, LUMP_MARKSURFACES, LUMP_MODELS, LUMP_NODES, LUMP_PLANES,
        LUMP_SURFEDGES, LUMP_TEXINFO, LUMP_TEXTURES, LUMP_VERTICES, LUMP_VISIBILITY, MAX_MAP_HULLS,
    },
    types::{
        Bsp, ClipNode, Edge, Entity, Face, IResult, Leaf, LightMap, LumpHeader, MarkSurface, Model,
        Node, Plane, SResult, SurfEdge, TexInfo, Texture, Vertex,
    },
    utils::{between_braces, quoted_text},
};

fn parse_lump_header(i: &[u8]) -> IResult<LumpHeader> {
    map(tuple((le_i32, le_i32)), |(offset, length)| LumpHeader {
        offset,
        length,
    })(i)
}

// parse_entity takes in &str, not &[u8]
// this is to make things more convenient to parse
fn parse_entity(i: &str) -> SResult<Entity> {
    let mut res = Entity::new();

    let parser = |i| delimited(multispace0, quoted_text, multispace0)(i);

    let (i, list) = all_consuming(many0(tuple((parser, parser))))(i)?;

    list.into_iter().for_each(|(key, value)| {
        res.insert(key.to_string(), value.to_string());
    });

    Ok((i, res))
}

// hacky stuffs to avoid parsing bytes :DD
fn parse_entities(i: &[u8]) -> IResult<Vec<Entity>> {
    let s = from_utf8(i);

    if s.is_err() {
        return context("Cannot interpret entity as utf8", fail)(i);
    }

    let parse_res = many0(between_braces(parse_entity))(s.unwrap());

    if parse_res.is_err() {
        return context("Cannot parse entities", fail)(i);
    }

    let (i, res) = parse_res.unwrap();

    Ok((i.as_bytes(), res))
}

fn parse_plane(i: &[u8]) -> IResult<Plane> {
    map(
        tuple((le_f32, le_f32, le_f32, le_f32, le_i32)),
        |(x, y, z, distance, type_)| Plane {
            normal: Vec3::new(x, y, z),
            distance,
            type_: type_.try_into().unwrap(),
        },
    )(i)
}

fn parse_planes(i: &[u8]) -> IResult<Vec<Plane>> {
    all_consuming(many0(parse_plane))(i)
}

fn parse_textures(i: &[u8]) -> IResult<Vec<Texture>> {
    let (header, tex_count) = le_u32(i)?;
    let (_, offsets) = count(le_i32, tex_count as usize)(header)?;

    let mut miptexes: Vec<Texture> = vec![];

    for offset in offsets {
        let (_, res) = parse_miptex(&i[(offset as usize)..])?;

        miptexes.push(res);
    }

    Ok((&[], miptexes))
}

fn parse_vertices(i: &[u8]) -> IResult<Vec<Vertex>> {
    all_consuming(many0(map(tuple((le_f32, le_f32, le_f32)), |(x, y, z)| {
        Vec3::new(x, y, z)
    })))(i)
}

fn parse_node(i: &[u8]) -> IResult<Node> {
    map(
        tuple((
            le_u32,
            le_i16,
            le_i16,
            count(le_i16, 3),
            count(le_i16, 3),
            le_u16,
            le_u16,
        )),
        |(plane, child1, child2, mins, maxs, first_face, face_count)| Node {
            plane,
            children: [child1, child2],
            mins: [mins[0], mins[1], mins[2]],
            maxs: [maxs[0], maxs[1], maxs[2]],
            first_face,
            face_count,
        },
    )(i)
}

fn parse_nodes(i: &[u8]) -> IResult<Vec<Node>> {
    all_consuming(many0(parse_node))(i)
}

fn parse_texinfo_singular(i: &[u8]) -> IResult<TexInfo> {
    map(
        tuple((
            count(le_f32, 3),
            le_f32,
            count(le_f32, 3),
            le_f32,
            le_u32,
            le_u32,
        )),
        |(u, u_offset, v, v_offset, texture_index, flags)| TexInfo {
            u: Vec3::from_slice(u.as_slice()),
            u_offset,
            v: Vec3::from_slice(v.as_slice()),
            v_offset,
            texture_index,
            flags,
        },
    )(i)
}

fn parse_texinfo(i: &[u8]) -> IResult<Vec<TexInfo>> {
    all_consuming(many0(parse_texinfo_singular))(i)
}

fn parse_face(i: &[u8]) -> IResult<Face> {
    map(
        tuple((
            le_u16,
            le_u16,
            le_i32,
            le_u16,
            le_u16,
            count(le_u8, 4),
            le_i32,
        )),
        |(plane, side, first_edge, edge_count, texinfo, styles, lightmap_offset)| Face {
            plane,
            side,
            first_edge,
            edge_count,
            texinfo,
            styles: [styles[0], styles[1], styles[2], styles[3]],
            lightmap_offset,
        },
    )(i)
}

fn parse_faces(i: &[u8]) -> IResult<Vec<Face>> {
    all_consuming(many0(parse_face))(i)
}

fn parse_lightmap(i: &[u8]) -> IResult<LightMap> {
    // map with zero lightmap will have lump with size of 1
    if i.len() == 1 {
        return Ok((&[], vec![]));
    }

    all_consuming(many0(map(count(le_u8, 3), |lightmap| {
        [lightmap[0], lightmap[1], lightmap[2]]
    })))(i)
}

fn parse_clipnode(i: &[u8]) -> IResult<ClipNode> {
    map(
        tuple((le_i32, le_i16, le_i16)),
        |(plane, child1, child2)| ClipNode {
            plane,
            children: [child1, child2],
        },
    )(i)
}

fn parse_clipnodes(i: &[u8]) -> IResult<Vec<ClipNode>> {
    all_consuming(many0(parse_clipnode))(i)
}

fn parse_leaf(i: &[u8]) -> IResult<Leaf> {
    map(
        tuple((
            le_i32,
            le_i32,
            count(le_i16, 3),
            count(le_i16, 3),
            le_u16,
            le_u16,
            count(le_u8, 4),
        )),
        |(
            contents,
            vis_offset,
            mins,
            maxs,
            first_mark_surface,
            mark_surface_count,
            ambient_levels,
        )| Leaf {
            contents: contents.try_into().unwrap(),
            vis_offset,
            mins: [mins[0], mins[1], mins[2]],
            maxs: [maxs[0], maxs[1], maxs[2]],
            first_mark_surface,
            mark_surface_count,
            ambient_levels: [
                ambient_levels[0],
                ambient_levels[1],
                ambient_levels[2],
                ambient_levels[3],
            ],
        },
    )(i)
}

fn parse_leaves(i: &[u8]) -> IResult<Vec<Leaf>> {
    all_consuming(many0(parse_leaf))(i)
}

fn parse_mark_surfaces(i: &[u8]) -> IResult<Vec<MarkSurface>> {
    all_consuming(many0(le_u16))(i)
}

fn parse_edges(i: &[u8]) -> IResult<Vec<Edge>> {
    all_consuming(many0(map(tuple((le_u16, le_u16)), |(p1, p2)| [p1, p2])))(i)
}

fn parse_surf_edges(i: &[u8]) -> IResult<Vec<SurfEdge>> {
    all_consuming(many0(le_i32))(i)
}

fn parse_model(i: &[u8]) -> IResult<Model> {
    map(
        tuple((
            count(le_f32, 3),
            count(le_f32, 3),
            count(le_f32, 3),
            count(le_i32, MAX_MAP_HULLS),
            le_i32,
            le_i32,
            le_i32,
        )),
        |(mins, maxs, origin, head_nodes, vis_leaves_count, first_face, face_count)| Model {
            mins: Vec3::new(mins[0], mins[1], mins[2]),
            maxs: Vec3::new(maxs[0], maxs[1], maxs[2]),
            origin: Vec3::new(origin[0], origin[1], origin[2]),
            head_nodes: [head_nodes[0], head_nodes[1], head_nodes[2], head_nodes[3]],
            vis_leaves_count,
            first_face,
            face_count,
        },
    )(i)
}

fn parse_models(i: &[u8]) -> IResult<Vec<Model>> {
    all_consuming(many0(parse_model))(i)
}

pub fn parse_bsp(i: &[u8]) -> IResult<Bsp> {
    let (beginning, version) = le_i32(i)?;

    if version != BSP_VERSION {
        return context(
            format!("Bsp Version is not 30: {}", BSP_VERSION).leak(),
            fail,
        )(i);
    }

    let (_, lumps) = count(parse_lump_header, HEADER_LUMPS)(beginning)?;

    let lump_section = |idx: usize| {
        &i[(lumps[idx].offset as usize)..((lumps[idx].offset + lumps[idx].length) as usize)]
    };

    let (_, entities) = parse_entities(lump_section(LUMP_ENTITIES))?;
    let (_, planes) = parse_planes(lump_section(LUMP_PLANES))?;
    let (_, textures) = parse_textures(lump_section(LUMP_TEXTURES))?;
    let (_, vertices) = parse_vertices(lump_section(LUMP_VERTICES))?;
    // TODO
    let (_, visibility) = rest(lump_section(LUMP_VISIBILITY))?;
    let (_, nodes) = parse_nodes(lump_section(LUMP_NODES))?;
    let (_, texinfo) = parse_texinfo(lump_section(LUMP_TEXINFO))?;
    let (_, faces) = parse_faces(lump_section(LUMP_FACES))?;
    let (_, lightmap) = parse_lightmap(lump_section(LUMP_LIGHTING))?;
    let (_, clipnodes) = parse_clipnodes(lump_section(LUMP_CLIPNODES))?;
    let (_, leaves) = parse_leaves(lump_section(LUMP_LEAVES))?;
    let (_, mark_surfaces) = parse_mark_surfaces(lump_section(LUMP_MARKSURFACES))?;
    let (_, edges) = parse_edges(lump_section(LUMP_EDGES))?;
    let (_, surf_edges) = parse_surf_edges(lump_section(LUMP_SURFEDGES))?;
    let (_, models) = parse_models(lump_section(LUMP_MODELS))?;

    Ok((
        &[],
        Bsp {
            entities,
            planes,
            textures,
            vertices,
            visibility: visibility.to_vec(),
            nodes,
            texinfo,
            faces,
            lightmap,
            clipnodes,
            leaves,
            mark_surfaces,
            edges,
            surf_edges,
            models,
        },
    ))
}
