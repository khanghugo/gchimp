use glam::DVec3;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use smd::{Smd, Triangle};

use crate::err;

use super::constants::MAX_SMD_TRIANGLE;

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

/// Splits one SMD to multiple SMD if number of vertices exceeds the limit.
///
/// Funnily enough, it will be based of the triangle count because I am not sure waht I am doing.
pub fn maybe_split_smd(smd: &Smd) -> Vec<Smd> {
    let mut res: Vec<Smd> = vec![];

    // No triangles means no need to split so just use the original
    if smd.triangles.is_empty() {
        res.push(smd.clone());

        return res;
    }

    let old_triangles = &smd.triangles;

    let needed_smd = old_triangles.len() / MAX_SMD_TRIANGLE + 1;

    (0..needed_smd)
        .map(|index| Smd {
            nodes: smd.nodes.clone(),
            skeleton: smd.skeleton.clone(),
            triangles: old_triangles
                .chunks(MAX_SMD_TRIANGLE)
                .nth(index)
                .unwrap()
                .to_vec(),
            ..Default::default()
        })
        .collect()
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

pub fn with_selected_textures(smd: &Smd, textures: &[String]) -> eyre::Result<Smd> {
    if smd.triangles.is_empty() {
        return err!("Smd has no triangles.");
    }

    let mut new_smd = smd.without_triangles();

    smd.triangles
        .iter()
        .filter(|triangle| textures.contains(&triangle.material))
        .for_each(|triangle| {
            new_smd.add_triangle(triangle.clone());
        });

    Ok(new_smd)
}
