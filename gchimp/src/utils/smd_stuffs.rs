use std::collections::{HashMap, HashSet};

use glam::DVec3;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use smd::{Smd, Triangle};

use crate::err;

use super::{constants::MAX_SMD_VERTEX, misc::remove_texture_prefix};

pub fn source_smd_to_goldsrc_smd(smd: &Smd) -> Vec<Smd> {
    maybe_split_smd(smd)
        .into_par_iter()
        .map(|mut smd| {
            smd.triangles.iter_mut().for_each(|triangle| {
                // remove the Source part
                triangle
                    .vertices
                    .iter_mut()
                    .for_each(|vertex| vertex.source = None);

                // make the texture name no space
                triangle.material = triangle.material.replace(" ", "_");

                // make the texture name lower case
                triangle.material = triangle.material.to_lowercase();

                // goldsrc models need .bmp in the name
                if !triangle.material.ends_with(".bmp") {
                    triangle.material += ".bmp";
                }
            });
            smd
        })
        .collect()
}

// poorman's hash function
#[inline]
fn vertex_hash(vertex: &smd::Vertex) -> String {
    let pos = vertex.pos;
    let norm = vertex.norm;
    format!("{}{}{}{}{}{}", pos.x, pos.y, pos.z, norm.x, norm.y, norm.z)
}

/// Splits one SMD to multiple SMD if number of vertices exceeds the limit.
pub fn maybe_split_smd(smd: &Smd) -> Vec<Smd> {
    let mut res: Vec<Smd> = vec![];

    // No triangles means no need to split so just use the original
    if smd.triangles.is_empty() {
        res.push(smd.clone());

        return res;
    }

    let mut vertex_list: HashMap<String, smd::Vertex> = HashMap::new();

    for triangle in &smd.triangles {
        for vertex in &triangle.vertices {
            vertex_list.insert(vertex_hash(vertex), vertex.to_owned());
        }
    }

    let mut triangle_list = smd.triangles.clone();

    // the strategy is
    // traverse by triangle
    // test whether adding all of the vertex of the current triangle to the current mesh
    // will exceed the vertex count or not
    // if it does, make new mesh
    // if it does not, repeat
    // brought to you by DeepSeek

    let mut res: Vec<Smd> = vec![];

    while !triangle_list.is_empty() {
        // triangle cannot repeat
        let mut curr_smd_triangles: Vec<Triangle> = vec![];
        // vertex can repeat
        let mut curr_smd_vertices: HashSet<String> = HashSet::new();

        while let Some(curr_triangle) = triangle_list.pop() {
            let vert0_hash = vertex_hash(&curr_triangle.vertices[0]);
            let vert1_hash = vertex_hash(&curr_triangle.vertices[1]);
            let vert2_hash = vertex_hash(&curr_triangle.vertices[2]);

            curr_smd_vertices.insert(vert0_hash);
            curr_smd_vertices.insert(vert1_hash);
            curr_smd_vertices.insert(vert2_hash);

            if curr_smd_vertices.len() > MAX_SMD_VERTEX {
                // if after adding those 3 vertices and the vertex count is exceeded
                // return the triangle back to the list and we are done with the current smd
                triangle_list.push(curr_triangle);
                break;
            }

            curr_smd_triangles.push(curr_triangle);
        }

        let new_smd = Smd {
            nodes: smd.nodes.clone(),
            skeleton: smd.skeleton.clone(),
            triangles: curr_smd_triangles,
            ..Default::default()
        };

        res.push(new_smd);
    }

    res
}

pub fn find_centroid(smd: &Smd) -> Option<DVec3> {
    if smd.triangles.is_empty() {
        return None;
    }

    find_centroid_from_triangles(smd.triangles.as_slice())
}

pub fn find_centroid_from_triangles(triangles: &[Triangle]) -> Option<DVec3> {
    if triangles.is_empty() {
        return None;
    }

    Some(
        triangles
            .par_iter()
            .map(|triangle| {
                triangle
                    .vertices
                    .iter()
                    .fold(DVec3::default(), |acc, e| acc + e.pos)
            })
            .reduce(DVec3::default, |acc, e| acc + e)
            / triangles.len() as f64
            / 3.,
    )
}

/// Mutates the original smd
pub fn move_by(smd: &mut Smd, offset: DVec3) {
    if smd.triangles.is_empty() {
        return;
    }

    smd.triangles.iter_mut().for_each(|triangle| {
        triangle.vertices.iter_mut().for_each(|vertex| {
            vertex.pos += offset;
        })
    });
}

pub fn add_bitmap_extension_to_texture(smd: &mut Smd) {
    if smd.triangles.is_empty() {
        return;
    }

    smd.triangles
        .iter_mut()
        .for_each(|triangle| triangle.material += ".bmp");
}

pub fn remove_texture_prefix_smd(smd: &mut Smd) {
    if smd.triangles.is_empty() {
        return;
    }

    smd.triangles.iter_mut().for_each(|triangle| {
        triangle.material = remove_texture_prefix(triangle.material.as_str());
    })
}

pub fn with_selected_textures(smd: &Smd, textures: &[&String]) -> eyre::Result<Smd> {
    if smd.triangles.is_empty() {
        return err!("Smd has no triangles.");
    }

    let mut new_smd = smd.without_triangles();

    smd.triangles
        .iter()
        .filter(|triangle| textures.contains(&&triangle.material))
        .for_each(|triangle| {
            new_smd.add_triangle(triangle.clone());
        });

    Ok(new_smd)
}

pub fn find_mins_maxs(triangles: &[Triangle]) -> [[f64; 3]; 2] {
    let minx = triangles.iter().fold(f64::MAX, |acc, e| {
        acc.min(e.vertices[0].pos.x)
            .min(e.vertices[1].pos.x)
            .min(e.vertices[2].pos.x)
    });
    let miny = triangles.iter().fold(f64::MAX, |acc, e| {
        acc.min(e.vertices[0].pos.y)
            .min(e.vertices[1].pos.y)
            .min(e.vertices[2].pos.y)
    });
    let minz = triangles.iter().fold(f64::MAX, |acc, e| {
        acc.min(e.vertices[0].pos.z)
            .min(e.vertices[1].pos.z)
            .min(e.vertices[2].pos.z)
    });

    let maxx = triangles.iter().fold(f64::MIN, |acc, e| {
        acc.max(e.vertices[0].pos.x)
            .max(e.vertices[1].pos.x)
            .max(e.vertices[2].pos.x)
    });
    let maxy = triangles.iter().fold(f64::MIN, |acc, e| {
        acc.max(e.vertices[0].pos.y)
            .max(e.vertices[1].pos.y)
            .max(e.vertices[2].pos.y)
    });
    let maxz = triangles.iter().fold(f64::MIN, |acc, e| {
        acc.max(e.vertices[0].pos.z)
            .max(e.vertices[1].pos.z)
            .max(e.vertices[2].pos.z)
    });

    [[minx, miny, minz], [maxx, maxy, maxz]]
}

pub fn textures_used_in_triangles(triangles: &[Triangle]) -> HashSet<String> {
    triangles.iter().fold(HashSet::new(), |mut acc, e| {
        if !acc.contains(&e.material) {
            acc.insert(e.material.clone());
        }

        acc
    })
}
