use std::{path::PathBuf, str::from_utf8};

use glam::{DVec2, DVec3};
use image::GenericImageView;
use qc::Qc;
use rayon::prelude::*;
use smd::{Smd, Triangle, Vertex};

use eyre::eyre;

use crate::utils::{
    constants::{EPSILON, STUDIOMDL_ERROR_PATTERN},
    img_stuffs::{rgba8_to_8bpp, write_8bpp_to_file, GoldSrcBmp},
    run_bin::run_studiomdl,
    simple_calculs::{Plane3D, Polygon3D},
};

pub struct BLBH {
    smd_path: PathBuf,
    texture_path: PathBuf,
    options: BLBHOptions,
}

pub struct BLBHOptions {
    convert_texture: bool,
    convert_smd: bool,
    compile_model: bool,
    flat_shade: bool,
    studiomdl: String,
    #[cfg(target_os = "linux")]
    wineprefix: String,
}

impl Default for BLBHOptions {
    fn default() -> Self {
        Self {
            convert_texture: true,
            convert_smd: true,
            compile_model: true,
            flat_shade: true,
            studiomdl: Default::default(),
            #[cfg(target_os = "linux")]
            wineprefix: Default::default(),
        }
    }
}

const MINIMUM_SIZE: u32 = 512;

pub fn blender_lightmap_baker_helper(blbh: &BLBH) -> eyre::Result<()> {
    let BLBH {
        smd_path,
        texture_path,
        options,
    } = blbh;

    let mut smd = Smd::from_file(smd_path)?;
    let image = image::open(texture_path)?;

    let (width, height) = image.dimensions();

    let w_count = width.div_ceil(MINIMUM_SIZE);
    let h_count = height.div_ceil(MINIMUM_SIZE);

    let texture_file_name = texture_path.file_stem().unwrap().to_str().unwrap();
    let smd_file_name = smd_path.file_stem().unwrap().to_str().unwrap();

    // split the images
    if options.convert_texture {
        (0..w_count).into_par_iter().for_each(|w_block| {
            (0..h_count).into_par_iter().for_each(|h_block| {
                let start_width = w_block * MINIMUM_SIZE;
                let start_height = h_block * MINIMUM_SIZE;

                let curr_width = (width - start_width).min(MINIMUM_SIZE);
                let curr_height = (height - start_height).min(MINIMUM_SIZE);

                let curr_image = image.crop_imm(start_width, start_height, curr_width, curr_height);

                let GoldSrcBmp {
                    image,
                    palette,
                    dimensions,
                } = rgba8_to_8bpp(curr_image.to_rgba8()).unwrap();

                let out_file_name = format!("{}{}{}.bmp", texture_file_name, w_block, h_block);
                write_8bpp_to_file(
                    &image,
                    &palette,
                    dimensions,
                    texture_path.with_file_name(out_file_name),
                )
                .unwrap();
            })
        });
    }

    if !options.convert_smd {
        return Ok(());
    }

    // modify smd
    // original UV map is 1 texture so it should go from 0 to 1
    // find with (x, y) texture it is in
    // maybe there's a problem with UV is exactly 1 but let's hope not

    let epsilon_round = |i: f64| {
        // if i + EPSILON >= i.ceil() {
        //     i.ceil()
        // } else {
        //     i.floor()
        // }

        return i.floor();
    };

    let find_w_h_block = |uv: DVec2| {
        let w = epsilon_round(uv[0] * width as f64 / MINIMUM_SIZE as f64) as u32;
        let h = epsilon_round(uv[1] * height as f64 / MINIMUM_SIZE as f64) as u32;

        (w, h)
    };

    let width_uv = MINIMUM_SIZE as f64 / width as f64;
    let height_uv = MINIMUM_SIZE as f64 / height as f64;

    let clamp_uv = |uv: DVec2, block: (u32, u32)| {
        let min_u = block.0 as f64 * width_uv;
        let min_v = block.1 as f64 * height_uv;

        DVec2::new(
            uv.x.clamp(min_u + EPSILON, min_u + width_uv - EPSILON),
            uv.y.clamp(min_v + EPSILON, min_v + height_uv - EPSILON),
        )
    };

    let wrap_uv = |uv: DVec2, block: (u32, u32)| {
        // return uv;
        let uv = clamp_uv(uv, block);
        let u = uv[0] % width_uv;
        let v = uv[1] % height_uv;
        let u = u / width_uv;
        let v = v / height_uv;
        DVec2::new(u, v)
    };

    // each vertex needs to shrink inward by 2 pixels so there's no seam
    // if not, the texture would repeat and that means texture filtering
    // the shitty thing here would be that the color difference might just be a seam
    // but at least it isn't scaled up to a pixel
    const SHRINK_FACTOR: f64 = 1. - (1. / MINIMUM_SIZE as f64) * 2.;
    let shrink_uvs = |uvs: Vec<DVec2>| {
        let centroid = uvs.iter().fold(DVec2::ZERO, |acc, &e| acc + e) / 3.;

        uvs.iter()
            .map(|&uv| {
                let vector = uv - centroid;
                let vector = vector * SHRINK_FACTOR;
                vector + centroid
            })
            .collect::<Vec<DVec2>>()
    };

    // split all triangles inside `triangles` until it's empty
    // fairly simple algorithm, not very optimized
    let new_triangles = smd
        .triangles
        .into_iter()
        .flat_map(|to_split| {
            // check if triangle fits
            let v1 = find_w_h_block(to_split.vertices[0].uv);
            let v2 = find_w_h_block(to_split.vertices[1].uv);
            let v3 = find_w_h_block(to_split.vertices[2].uv);

            // if fits inside a texture, add it into the result and continue
            if v1.0 == v2.0 && v2.0 == v3.0 && v1.1 == v2.1 && v2.1 == v3.1 {
                let material = format!("{}{}{}.bmp", texture_file_name, v1.0, h_count - v1.1 - 1);
                let mut new_triangle = to_split.clone();
                new_triangle.material = material;
                new_triangle.vertices.iter_mut().for_each(|vertex| {
                    vertex.uv = wrap_uv(vertex.uv, v1);
                });

                return vec![new_triangle];
            }

            // now we do big stuffs
            let polygon = Polygon3D::from(vec![
                to_split.vertices[0].pos,
                to_split.vertices[1].pos,
                to_split.vertices[2].pos,
            ]);

            // dumb fuck this normal doesnt' do shit
            // let triangle_normal = to_split.vertices[0].norm;
            let triangle_normal = polygon.normal().unwrap().into();

            // converts a uv coordinate from a triangle to world coordinate
            // so world coordinate would be coplanar with the triangle
            let uv_to_world = |uv: DVec2| {
                // choose an anchor vertex
                // choose another vertex then we have a vector on uv plane
                // from the chosen uv coordinate, we will have another vector with the anchor vertex
                // now we have two vectors, the anchor vector and the uv vertex
                // find that displacement in uv space then translate that to world space
                let anchor_vertex = &to_split.vertices[0];
                let anchor_vector = to_split.vertices[1].pos - anchor_vertex.pos;
                let anchor_vector_uv = to_split.vertices[1].uv - anchor_vertex.uv;

                let target_vector_uv = uv - anchor_vertex.uv;
                let angle = anchor_vector_uv.angle_between(target_vector_uv);
                let scale = target_vector_uv.length() / anchor_vector_uv.length();

                let normal = anchor_vertex.norm;
                // rotate the anchor_vector around triangle normal
                // rodrigues' rotation
                let result_vector = anchor_vector * angle.cos()
                    + normal.cross(anchor_vector) * angle.sin()
                    + normal * (normal.dot(anchor_vector) * (1. - angle.cos()));
                let result_vector = result_vector * scale; // scale the vector to match
                let result_vector = result_vector + anchor_vertex.pos; // translate back to where it starts

                result_vector
            };

            // converts a world coordinate coplanar to a triangle into uv coordinate as used in the original triangle
            // this means the uv coordinate would be in the big texture
            // so we have to convert that back to smaller triangle coordinate again
            // the steps will mirror uv_to_world because we can select
            let world_to_uv = |p: DVec3| {
                let anchor_vertex = &to_split.vertices[0];
                let anchor_vector = to_split.vertices[1].pos - anchor_vertex.pos;
                let anchor_vector_uv = to_split.vertices[1].uv - anchor_vertex.uv;

                let target_vector = p - anchor_vertex.pos;
                let angle = anchor_vector.angle_between(target_vector);
                let scale = target_vector.length() / anchor_vector.length();

                // fucking dumb shit cannot do eigenvectors
                let angle = if angle.is_nan() { 0. } else { angle };

                let rotation_matrix = [[angle.cos(), -angle.sin()], [angle.sin(), angle.cos()]];
                let result_vector_uv_u = anchor_vector_uv.dot(rotation_matrix[0].into());
                let result_vector_uv_v = anchor_vector_uv.dot(rotation_matrix[1].into());
                let result_vector_uv = DVec2::new(result_vector_uv_u, result_vector_uv_v);
                let result_vector_uv = result_vector_uv * scale;
                let result_vector_uv = result_vector_uv + anchor_vertex.uv;

                result_vector_uv
            };

            // now, to get a cutting plane, we have to find the plane normal and a point on the plane
            // if we cut vertically, we can find two uv_to_world coordinates such that they are on the cutting plane
            // from two points, we will have a new plane that is orthorgonal to the triangle plane with the normal of that two points
            // so, we have to cross product of that horizontal plane with the triangle plane
            // and we will have normal of cutting plane
            // do dot product to find the distance of the plane
            // let vertical_cut_count = v1.0.max(v2.0).max(v3.0) - 1;
            // let horizontal_cut_count = v1.1.max(v2.1).max(v3.1) - 1;

            // TODO: calculate cut count instead of cut 16 times every time
            let mut polygon_res = vec![polygon];

            // cuts vertically
            // subtracts 1 because 2 blocks means 1 cut and so on
            (1..w_count).for_each(|w_block| {
                let u = w_block as f64 * width_uv;

                let v1 = uv_to_world((u, 0.).into());
                let v2 = uv_to_world((u, 1.).into());
                let orthogonal_plane_normal = v1 - v2;
                let cutting_plane_normal = orthogonal_plane_normal.cross(triangle_normal);
                let cutting_plane_distance = cutting_plane_normal.dot(v1);
                let plane = Plane3D::new(
                    cutting_plane_normal.x,
                    cutting_plane_normal.y,
                    cutting_plane_normal.z,
                    cutting_plane_distance,
                );

                polygon_res = polygon_res
                    .iter()
                    .flat_map(|polygon| polygon.split(&plane))
                    // sort vertices after first cut because i think something stupid in the split function
                    .map(|polygon| polygon.with_sorted_vertices().unwrap())
                    .collect();
            });

            // cuts horizontally
            (1..h_count).for_each(|h_block| {
                let v = h_block as f64 * height_uv;

                let v1 = uv_to_world((0., v).into());
                let v2 = uv_to_world((1., v).into());
                let orthogonal_plane_normal = v1 - v2;
                let cutting_plane_normal = orthogonal_plane_normal.cross(triangle_normal);
                let cutting_plane_distance = cutting_plane_normal.dot(v1);
                let plane = Plane3D::new(
                    cutting_plane_normal.x,
                    cutting_plane_normal.y,
                    cutting_plane_normal.z,
                    cutting_plane_distance,
                );

                polygon_res = polygon_res
                    .iter()
                    // triangulate will sort the vertices after so no need to sort vertices here
                    .flat_map(|polygon| polygon.split(&plane))
                    .collect();
            });

            // clear shits because i dont want to fix the split function
            polygon_res = polygon_res
                .into_iter()
                .filter(|e| e.vertices().len() >= 3)
                .collect();

            // triangulates
            polygon_res = polygon_res
                .into_iter()
                .flat_map(|polygon| {
                    // huh
                    let reverse = if polygon
                        .normal()
                        .unwrap()
                        .dot(triangle_normal.into())
                        .is_sign_negative()
                    {
                        true
                    } else {
                        false
                    };

                    polygon
                        .triangulate(reverse)
                        .expect("cannot triangulate triangles")
                })
                .map(|triangle| triangle.to_polygon())
                .collect();

            // these are polygons but they're specifically triangles so that's fine
            // convert these mathematical polygons into smd triangle

            let original_sin = &to_split.vertices[0];

            polygon_res
                .into_iter()
                .map(|polygon| {
                    // every polygon is a triangle so that's ok
                    // every vertices here are guaranteed to fit inside a texture
                    // but because of some funky math, to find where the triangle fits, we can use centroid
                    let centroid = polygon
                        .vertices()
                        .iter()
                        .fold(DVec2::ZERO, |acc, e| world_to_uv(e.into()) + acc)
                        / 3.;

                    let (w, h) = find_w_h_block(centroid);

                    let uvs = vec![
                        wrap_uv(world_to_uv(polygon.vertices()[0].into()), (w, h)),
                        wrap_uv(world_to_uv(polygon.vertices()[1].into()), (w, h)),
                        wrap_uv(world_to_uv(polygon.vertices()[2].into()), (w, h)),
                    ];

                    // scale the triangle so there's no seam
                    let uvs = shrink_uvs(uvs);

                    let v0 = Vertex {
                        parent: original_sin.parent,
                        pos: polygon.vertices()[0].into(),
                        norm: original_sin.norm,
                        uv: uvs[0],
                        source: None,
                    };

                    let v1 = Vertex {
                        parent: original_sin.parent,
                        pos: polygon.vertices()[1].into(),
                        norm: original_sin.norm,
                        uv: uvs[1],
                        source: None,
                    };

                    let v2 = Vertex {
                        parent: original_sin.parent,
                        pos: polygon.vertices()[2].into(),
                        norm: original_sin.norm,
                        uv: uvs[2],
                        source: None,
                    };

                    let h = h_count - h - 1; // the texture is exported from top left while uv is bottom left
                    let material = format!("{}{}{}.bmp", texture_file_name, w, h);

                    Triangle {
                        material,
                        vertices: vec![v0, v1, v2],
                    }
                })
                .collect::<Vec<Triangle>>()
        })
        .collect::<Vec<Triangle>>();

    smd.triangles = new_triangles;

    smd.write(smd_path.with_file_name(format!("{}_blbh.smd", smd_file_name)))?;

    if options.compile_model {
        let idle_smd = Smd::new_basic();
        let smd_root = smd_path.parent().unwrap();
        let texture_file_root = smd_path.parent().unwrap();

        idle_smd.write(smd_path.with_file_name("idle.smd"))?;

        let mut qc = Qc::new_basic();
        qc.set_model_name(
            smd_path
                .with_file_name(format!("{}_blbh.mdl", smd_file_name))
                .to_str()
                .unwrap(),
        );
        qc.set_cd(smd_root.to_str().unwrap());
        qc.set_cd_texture(texture_file_root.to_str().unwrap());

        if options.flat_shade {
            (0..w_count).for_each(|w_block| {
                (0..h_count).for_each(|h_block| {
                    qc.add_texrendermode(
                        format!("{}{}{}.bmp", texture_file_name, w_block, h_block).as_str(),
                        qc::RenderMode::FlatShade,
                    );
                })
            });
        }

        qc.add_body(
            "studio0",
            format!("{}_blbh", smd_file_name).as_str(),
            false,
            None,
        );
        qc.add_sequence("idle", "idle", vec![]);

        let qc_path = smd_path.with_file_name(format!("{}_blbh.qc", smd_file_name));
        qc.write(qc_path.as_path())?;

        // run studiomdl
        #[cfg(target_os = "windows")]
        let handle = run_studiomdl(
            qc_path.as_path(),
            PathBuf::from(options.studiomdl.as_str()).as_path(),
        );

        #[cfg(target_os = "linux")]
        let handle = run_studiomdl(
            qc_path.as_path(),
            PathBuf::from(options.studiomdl.as_str()).as_path(),
            options.wineprefix.as_str(),
        );

        match handle.join() {
            Ok(res) => {
                let output = res?;
                let stdout = from_utf8(&output.stdout).unwrap();

                let maybe_err = stdout.find(STUDIOMDL_ERROR_PATTERN);

                if let Some(err_index) = maybe_err {
                    let err = stdout[err_index + STUDIOMDL_ERROR_PATTERN.len()..].to_string();
                    let err_str = format!("cannot compile: {}", err.trim());
                    return Err(eyre!(err_str));
                }
            }
            Err(_) => {
                let err_str = "No idea what happens with running studiomdl. Probably just a dream.";

                return Err(eyre!(err_str));
            }
        };
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn options() -> BLBHOptions {
        BLBHOptions {
            convert_texture: false,
            convert_smd: true,
            compile_model: true,
            flat_shade: true,
            studiomdl: String::from("/home/khang/gchimp/dist/studiomdl.exe"),
            #[cfg(target_os = "linux")]
            wineprefix: String::from("/home/khang/.local/share/wineprefixes/wine32/"),
        }
    }

    #[test]
    fn convert_texture() {
        let smd_path = "/home/khang/gchimp/examples/blbh/cube_4k.smd";
        let texture_path = "/home/khang/gchimp/examples/blbh/cube_1k.png";

        let blbh = BLBH {
            smd_path: smd_path.into(),
            texture_path: texture_path.into(),
            options: options(),
        };
        blender_lightmap_baker_helper(&blbh).unwrap();
    }

    #[test]
    fn convert_smd() {
        let smd_path = "/home/khang/gchimp/examples/blbh/cube_4k.smd";
        let texture_path = "/home/khang/gchimp/examples/blbh/cube_1k.png";

        let blbh = BLBH {
            smd_path: smd_path.into(),
            texture_path: texture_path.into(),
            options: options(),
        };
        blender_lightmap_baker_helper(&blbh).unwrap();
    }

    #[test]
    fn convert_4k() {
        let smd_path = "/home/khang/gchimp/examples/blbh/cube_4k.smd";
        let texture_path = "/home/khang/gchimp/examples/blbh/cube_4k.png";

        let blbh = BLBH {
            smd_path: smd_path.into(),
            texture_path: texture_path.into(),
            options: options(),
        };
        blender_lightmap_baker_helper(&blbh).unwrap();
    }

    #[test]
    fn convert_scene_4k() {
        let smd_path = "/home/khang/gchimp/examples/blbh/scene_4k.smd";
        let texture_path = "/home/khang/gchimp/examples/blbh/scene_4k.png";

        let blbh = BLBH {
            smd_path: smd_path.into(),
            texture_path: texture_path.into(),
            options: options(),
        };
        blender_lightmap_baker_helper(&blbh).unwrap();
    }

    #[test]
    fn convert_minimum() {
        let smd_path = "/home/khang/gchimp/examples/blbh/minimum.smd";
        let texture_path = "/home/khang/gchimp/examples/blbh/scene_4k.png";

        let blbh = BLBH {
            smd_path: smd_path.into(),
            texture_path: texture_path.into(),
            options: options(),
        };
        blender_lightmap_baker_helper(&blbh).unwrap();
    }
}
