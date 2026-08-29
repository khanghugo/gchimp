use std::collections::{HashMap, HashSet};

use glam::DVec3;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use smd::{Smd, Triangle};

use crate::err;

use super::misc::remove_texture_prefix;
use common::constants::MAX_SMD_VERTEX;

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
pub fn maybe_split_smd(smd: &Smd) -> Vec<Smd> {
    // No triangles means no need to split so just use the original
    if smd.triangles.is_empty() {
        return vec![smd.clone()];
    }

    let triangle_list = smd.triangles.clone();
    let split_triangles = maybe_split_triangles(triangle_list);

    split_triangles
        .into_iter()
        .map(|triangles| Smd {
            nodes: smd.nodes.clone(),
            skeleton: smd.skeleton.clone(),
            triangles,
            ..Default::default()
        })
        .collect()
}

pub fn maybe_split_triangles(mut triangles: Vec<Triangle>) -> Vec<Vec<Triangle>> {
    let mut vertex_list: HashMap<String, smd::Vertex> = HashMap::new();

    for triangle in &triangles {
        for vertex in &triangle.vertices {
            vertex_list.insert(vertex.bad_hash(), vertex.to_owned());
        }
    }

    let mut res = vec![];

    while !triangles.is_empty() {
        // triangle cannot repeat
        let mut curr_smd_triangles: Vec<Triangle> = vec![];
        // vertex can repeat
        let mut curr_smd_vertices: HashSet<String> = HashSet::new();
        let mut curr_smd_normals: Vec<String> = vec![];

        while let Some(curr_triangle) = triangles.pop() {
            let vert0_hash = curr_triangle.vertices[0].bad_pos_hash();
            let vert1_hash = curr_triangle.vertices[1].bad_pos_hash();
            let vert2_hash = curr_triangle.vertices[2].bad_pos_hash();

            let norm0_hash = curr_triangle.vertices[0].bad_norm_hash();
            let norm1_hash = curr_triangle.vertices[1].bad_norm_hash();
            let norm2_hash = curr_triangle.vertices[2].bad_norm_hash();

            curr_smd_vertices.insert(vert0_hash);
            curr_smd_vertices.insert(vert1_hash);
            curr_smd_vertices.insert(vert2_hash);

            curr_smd_normals.push(norm0_hash);
            curr_smd_normals.push(norm1_hash);
            curr_smd_normals.push(norm2_hash);

            if curr_smd_vertices.len() >= MAX_SMD_VERTEX || curr_smd_normals.len() >= MAX_SMD_VERTEX
            {
                // if after adding those 3 vertices and the vertex count is exceeded
                // return the triangle back to the list and we are done with the current smd
                triangles.push(curr_triangle);
                break;
            }

            curr_smd_triangles.push(curr_triangle);
        }

        res.push(curr_smd_triangles);
    }

    res
}

pub fn find_centroid(smd: &Smd) -> Option<DVec3> {
    if smd.triangles.is_empty() {
        return None;
    }

    find_centroid_from_triangles(smd.triangles.as_slice())
}

// there is a problem with this function and that is it is tessellation biases
// for many mesh runs, vertices are shared and that is counted in this average function
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

pub fn find_aabb_center_from_triangles(triangles: &[Triangle]) -> Option<DVec3> {
    if triangles.is_empty() {
        return None;
    }

    let mut min = DVec3::MAX;
    let mut max = DVec3::MIN;

    for tri in triangles {
        for v in &tri.vertices {
            min = min.min(v.pos);
            max = max.max(v.pos);
        }
    }

    Some((min + max) * 0.5)
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

pub fn find_mins_maxs(triangles: &[Triangle]) -> (DVec3, DVec3) {
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

    ([minx, miny, minz].into(), [maxx, maxy, maxz].into())
}

pub fn textures_used_in_triangles(triangles: &[Triangle]) -> HashSet<String> {
    triangles.iter().fold(HashSet::new(), |mut acc, e| {
        if !acc.contains(&e.material) {
            acc.insert(e.material.clone());
        }

        acc
    })
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, VecDeque},
        hash::{DefaultHasher, Hash, Hasher},
        path::Path,
    };

    use glam::DVec3;
    use rand::{RngExt, SeedableRng, rngs::StdRng};
    use smd::{Smd, Triangle};

    const SMD_PATH: &str = "/WD3/cube(2).smd";

    fn seed_from_string(seed_str: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        seed_str.hash(&mut hasher);
        hasher.finish()
    }

    fn quantize_pos(pos: DVec3) -> (i64, i64, i64) {
        let scale = 1000.0; // 1mm grid snapping
        (
            (pos.x * scale).round() as i64,
            (pos.y * scale).round() as i64,
            (pos.z * scale).round() as i64,
        )
    }

    fn quantize_val(val: f64) -> i64 {
        (val * 1000.0).round() as i64
    }

    #[test]
    fn randomize_normal() {
        let mut smd = Smd::from_file(SMD_PATH).unwrap();

        let get_rand_dir = |seed: u64| {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            rng.random::<f64>();

            loop {
                let v = DVec3::new(
                    rng.random::<f64>() * 2.0 - 1.0,
                    rng.random::<f64>() * 2.0 - 1.0,
                    rng.random::<f64>() * 2.0 - 1.0,
                );
                // Ensure point is inside unit sphere to prevent corner bias
                if v.length_squared() <= 1.0 && v.length_squared() > 1e-12 {
                    return v.normalize();
                }
            }
        };

        smd.triangles.iter_mut().for_each(|triangle| {
            triangle.vertices.iter_mut().for_each(|vertex| {
                let seed = seed_from_string(&vertex.bad_norm_hash());
                vertex.norm = get_rand_dir(seed);
            });
        });

        smd.write(Path::new(SMD_PATH).with_file_name("randomized.smd"))
            .unwrap();
    }

    fn quantize_vec(v: DVec3) -> (i64, i64, i64) {
        let scale = 1000.0;
        (
            (v.x * scale).round() as i64,
            (v.y * scale).round() as i64,
            (v.z * scale).round() as i64,
        )
    }

    #[test]
    fn randomize_normal_connected_faces() {
        let mut smd = Smd::from_file(SMD_PATH).unwrap();

        let get_rand_dir = |seed: u64| {
            let mut rng = StdRng::seed_from_u64(seed);
            loop {
                let v = DVec3::new(
                    rng.random::<f64>() * 2.0 - 1.0,
                    rng.random::<f64>() * 2.0 - 1.0,
                    rng.random::<f64>() * 2.0 - 1.0,
                );
                let len_sq = v.length_squared();
                if len_sq <= 1.0 && len_sq > 1e-12 {
                    return v.normalize();
                }
            }
        };

        // --- STEP 1: Find which triangles share edges (2 vertices) ---
        // We use edges rather than single vertices so we don't accidentally link diagonal corners
        let mut edge_to_triangles: HashMap<((i64, i64, i64), (i64, i64, i64)), Vec<usize>> =
            HashMap::new();

        for (tri_idx, triangle) in smd.triangles.iter().enumerate() {
            let p0 = quantize_vec(triangle.vertices[0].pos);
            let p1 = quantize_vec(triangle.vertices[1].pos);
            let p2 = quantize_vec(triangle.vertices[2].pos);

            // Helper to sort edge coordinates so (A, B) matches (B, A)
            let mut add_edge = |mut a, mut b| {
                if a > b {
                    std::mem::swap(&mut a, &mut b);
                }
                edge_to_triangles.entry((a, b)).or_default().push(tri_idx);
            };

            add_edge(p0, p1);
            add_edge(p1, p2);
            add_edge(p2, p0);
        }

        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); smd.triangles.len()];
        for tri_indices in edge_to_triangles.values() {
            for &i in tri_indices {
                for &j in tri_indices {
                    if i != j {
                        adj[i].push(j);
                    }
                }
            }
        }

        // Helper: Gets the average original normal of a triangle
        let get_tri_norm = |tri: &Triangle| {
            let norm = (tri.vertices[0].norm + tri.vertices[1].norm + tri.vertices[2].norm) / 3.0;
            quantize_vec(norm)
        };

        // --- STEP 2: Flood-fill to find connected "Faces" ---
        let mut visited = vec![false; smd.triangles.len()];
        let mut faces: Vec<Vec<usize>> = Vec::new();

        for i in 0..smd.triangles.len() {
            if visited[i] {
                continue;
            }

            let mut face = Vec::new();
            let mut queue = VecDeque::new();

            let base_norm = get_tri_norm(&smd.triangles[i]);

            queue.push_back(i);
            visited[i] = true;

            while let Some(tri_idx) = queue.pop_front() {
                face.push(tri_idx);

                for &neighbor in &adj[tri_idx] {
                    if !visited[neighbor] {
                        let neighbor_norm = get_tri_norm(&smd.triangles[neighbor]);

                        // CORE LOGIC: Must share vertices AND have the same starting normal!
                        if base_norm == neighbor_norm {
                            visited[neighbor] = true;
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
            faces.push(face);
        }

        // --- STEP 3: Assign one random normal per Face island ---
        for face in faces {
            // Find the physical center of this face to use as its unique seed.
            // Because Ring 1 and Ring 2 are in different positions, their centers will create different seeds.
            let mut face_centroid = DVec3::ZERO;
            let mut count = 0.0;

            for &tri_idx in &face {
                for v in &smd.triangles[tri_idx].vertices {
                    face_centroid += v.pos;
                    count += 1.0;
                }
            }
            face_centroid /= count;

            let seed_str = format!("{:?}", quantize_vec(face_centroid));
            let seed = seed_from_string(&seed_str);

            let new_norm = get_rand_dir(seed);

            for &tri_idx in &face {
                let triangle = &mut smd.triangles[tri_idx];
                for vertex in &mut triangle.vertices {
                    vertex.norm = new_norm; // or apply jitter here
                }
            }
        }

        // print!("")

        smd.write(Path::new(SMD_PATH).with_file_name("randomized_faces.smd"))
            .unwrap();
    }
    #[test]
    fn jitter_normal() {
        let mut smd = Smd::from_file(SMD_PATH).unwrap();

        // Scale factor for the jitter (e.g., 0.1 = subtle, 0.25 = moderate)
        let jitter_amount = 0.20;

        // Generates a float between -0.5 and 0.5
        let get_rand_offset = || (rand::random::<f64>() - 0.5) * jitter_amount;

        smd.triangles.iter_mut().for_each(|triangle| {
            let offset = DVec3::new(get_rand_offset(), get_rand_offset(), get_rand_offset());

            triangle.vertices.iter_mut().for_each(|vertex| {
                // Add small offset to the original normal, then normalize back to length 1.0
                vertex.norm = (vertex.norm + offset).normalize();
            });
        });

        smd.write(Path::new(SMD_PATH).with_file_name("jittered.smd"))
            .unwrap();
    }
}
