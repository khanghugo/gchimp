use std::{path::PathBuf, str::from_utf8};

use glam::{DVec2, DVec3};
use image::GenericImageView;
use qc::Qc;
use rayon::prelude::*;
use smd::{Smd, Triangle, Vertex};

use eyre::eyre;

use crate::{
    err,
    utils::{
        constants::{EPSILON, STUDIOMDL_ERROR_PATTERN},
        img_stuffs::{rgba8_to_8bpp, write_8bpp_to_file, GoldSrcBmp},
        simple_calculs::{Matrix2x2, Plane3D, Polygon3D},
        smd_stuffs::{maybe_split_smd, textures_used_in_triangles},
    },
};

#[cfg(target_arch = "x86_64")]
use crate::utils::run_bin::run_studiomdl;

pub struct BLBH {
    pub smd_path: PathBuf,
    pub texture_path: PathBuf,
    pub options: BLBHOptions,
}

#[derive(Debug, Clone)]
pub struct BLBHOptions {
    pub convert_texture: bool,
    pub convert_smd: bool,
    pub compile_model: bool,
    pub flat_shade: bool,
    pub uv_clamp_factor: f32,
    // pub origin: DVec3,
    pub studiomdl: String,
    #[cfg(target_os = "linux")]
    pub wineprefix: String,
}

impl Default for BLBHOptions {
    fn default() -> Self {
        Self {
            convert_texture: true,
            convert_smd: true,
            compile_model: true,
            flat_shade: true,
            uv_clamp_factor: BLBH_DEFAULT_UV_CLAMP_FACTOR,
            // origin: DVec3::ZERO,
            studiomdl: Default::default(),
            #[cfg(target_os = "linux")]
            wineprefix: Default::default(),
        }
    }
}

pub const BLBH_DEFAULT_UV_CLAMP_FACTOR: f32 = 0.001953125;

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

    let epsilon_round = |i: f64| i.floor();

    let find_w_h_block = |uv: DVec2| {
        let w = epsilon_round(uv[0] * width as f64 / MINIMUM_SIZE as f64) as u32;
        let h = epsilon_round(uv[1] * height as f64 / MINIMUM_SIZE as f64) as u32;

        (w, h)
    };

    let width_uv = MINIMUM_SIZE as f64 / width as f64;
    let height_uv = MINIMUM_SIZE as f64 / height as f64;

    let find_uv = |uv: DVec2, block: (u32, u32)| {
        let min_u = block.0 as f64 * width_uv;
        let min_v = block.1 as f64 * height_uv;

        DVec2::new(
            uv.x.clamp(min_u + EPSILON, min_u + width_uv - EPSILON),
            uv.y.clamp(min_v + EPSILON, min_v + height_uv - EPSILON),
        )
    };

    let wrap_uv = |uv: DVec2, block: (u32, u32)| {
        // find the uv block
        let uv = find_uv(uv, block);

        // normalize the uv based on the partitioned texture
        let u = uv[0] % width_uv;
        let v = uv[1] % height_uv;
        let u = u / width_uv;
        let v = v / height_uv;

        // clamping the uv to avoid linear filtering repeating texture
        let u = u.clamp(
            0. + blbh.options.uv_clamp_factor as f64,
            1. - blbh.options.uv_clamp_factor as f64,
        );
        let v = v.clamp(
            0. + blbh.options.uv_clamp_factor as f64,
            1. - blbh.options.uv_clamp_factor as f64,
        );
        DVec2::new(u, v)
    };

    // check if uv is unwrapped properly
    if smd.triangles.iter().any(|triangle| {
        let is_outside = |x| !(0. ..=1.).contains(&x);
        triangle
            .vertices
            .iter()
            .any(|vertex| is_outside(vertex.uv.x) || is_outside(vertex.uv.y))
    }) {
        return err!("mesh uv is outside [0, 1]");
    }

    // split all triangles inside `triangles` until it's empty
    // fairly simple algorithm, not very optimized
    let new_triangles = smd
        .triangles
        .into_iter()
        .flat_map(|to_split| {
            // if the material does not match the image file, don't process the triangle
            if to_split.material != texture_file_name {
                return vec![to_split];
            }

            // our polygon
            let polygon = Polygon3D::from(vec![
                to_split.vertices[0].pos,
                to_split.vertices[1].pos,
                to_split.vertices[2].pos,
            ]);

            let anchor_vertex = &to_split.vertices[0];
            let anchor_vector_uv0 = to_split.vertices[1].uv - anchor_vertex.uv;
            let anchor_vector_uv1 = to_split.vertices[2].uv - anchor_vertex.uv;
            let anchor_vector_pos0 = to_split.vertices[1].pos - anchor_vertex.pos;
            let anchor_vector_pos1 = to_split.vertices[2].pos - anchor_vertex.pos;

            let uv_mat = Matrix2x2::from([
                anchor_vector_uv0.x,
                anchor_vector_uv1.x,
                anchor_vector_uv0.y,
                anchor_vector_uv1.y,
            ]);

            // check if triangle fits
            let v1 = find_w_h_block(to_split.vertices[0].uv);
            let v2 = find_w_h_block(to_split.vertices[1].uv);
            let v3 = find_w_h_block(to_split.vertices[2].uv);
            let fits_in_one_block = v1 == v2 && v2 == v3;

            // check if the triangle is degenerate
            let is_degenerate = uv_mat.determinant().abs() < EPSILON;

            // if edge case, skip
            if fits_in_one_block || is_degenerate {
                let weird_overflow = h_count.saturating_sub(v1.1).saturating_sub(1);

                let material = format!("{}{}{}.bmp", texture_file_name, v1.0, weird_overflow);
                let mut new_triangle = to_split.clone();
                new_triangle.material = material;
                new_triangle.vertices.iter_mut().for_each(|vertex| {
                    vertex.uv = wrap_uv(vertex.uv, v1);
                });

                return vec![new_triangle];
            }

            // dumb fuck this normal doesnt' do shit
            // let triangle_normal = to_split.vertices[0].norm;
            let triangle_normal: DVec3 = polygon.normal().unwrap().into();
            let triangle_normal = triangle_normal.normalize();

            // converts a uv coordinate from a triangle to world coordinate
            // so world coordinate would be coplanar with the triangle
            // represent the uv coordinate with the basis of two vectors
            let uv_to_world = |uv: DVec2| {
                let target_vector_uv = uv - anchor_vertex.uv;

                let coefficients: [f64; 2] = uv_mat
                    .solve_cramer([target_vector_uv.x, target_vector_uv.y])
                    .unwrap_or_else(|_| {
                        panic!(
                            "cannot solve by cramer's rule {} {} {:?} {:?}",
                            anchor_vector_uv0,
                            anchor_vector_uv1,
                            [target_vector_uv.x, target_vector_uv.y],
                            to_split
                        )
                    });

                (anchor_vector_pos0 * coefficients[0] + anchor_vector_pos1 * coefficients[1])
                    + anchor_vertex.pos
            };

            let world_to_uv = |p: DVec3| {
                let target_vector_pos = p - anchor_vertex.pos;

                // need to solve cramer's rule 3 times
                let coefficients = if let Ok(res) = Matrix2x2::from([
                    anchor_vector_pos0.y,
                    anchor_vector_pos1.y,
                    anchor_vector_pos0.z,
                    anchor_vector_pos1.z,
                ])
                .solve_cramer([target_vector_pos.y, target_vector_pos.z])
                {
                    res
                } else if let Ok(res) = Matrix2x2::from([
                    anchor_vector_pos0.x,
                    anchor_vector_pos1.x,
                    anchor_vector_pos0.z,
                    anchor_vector_pos1.z,
                ])
                .solve_cramer([target_vector_pos.x, target_vector_pos.z])
                {
                    res
                } else if let Ok(res) = Matrix2x2::from([
                    anchor_vector_pos0.x,
                    anchor_vector_pos1.x,
                    anchor_vector_pos0.y,
                    anchor_vector_pos1.y,
                ])
                .solve_cramer([target_vector_pos.x, target_vector_pos.y])
                {
                    res
                } else {
                    unreachable!("cannot solve by cramer's rule. will this come back bite me in the ass like the other case?")
                };

                (anchor_vector_uv0 * coefficients[0] + anchor_vector_uv1 * coefficients[1])
                    + anchor_vertex.uv
            };

            // now, to get a cutting plane, we have to find the plane normal and a point on the plane
            // if we cut vertically, we can find two uv_to_world coordinates such that they are on the cutting plane
            // from two points, we will have a new plane that is orthorgonal to the triangle plane with the normal of that two points
            // so, we have to cross product of that horizontal plane with the triangle plane
            // and we will have normal of cutting plane
            // do dot product to find the distance of the plane
            let min_w = v1.0.min(v2.0).min(v3.0);
            let max_w = v1.0.max(v2.0).max(v3.0);
            let min_h = v1.1.min(v2.1).min(v3.1);
            let max_h = v1.1.max(v2.1).max(v3.1);

            let mut polygon_res = vec![polygon];

            // cuts vertically
            // subtracts 1 because 2 blocks means 1 cut and so on
            (min_w..max_w).for_each(|w_block| {
                // if we have triangle covering betwen 0 and 1, we only want the cut to start from 1
                let w_block = w_block + 1;
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
                    .flat_map(|polygon| polygon.split3(&plane))
                    // that means chatgpt copy pasted result does not work very good
                    // but anyways
                    .filter(|polygon| polygon.vertices().len() >= 3)
                    // sort vertices after first cut because i think something stupid in the split function
                    .map(|polygon| polygon.with_sorted_vertices().unwrap())
                    .collect();
            });

            // cuts horizontally
            (min_h..max_h).for_each(|h_block| {
                let h_block = h_block + 1;
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
                    .flat_map(|polygon| polygon.split3(&plane))
                    .collect();
            });

            // clear shits because i dont want to fix the split function
            polygon_res.retain(|e| e.vertices().len() >= 3);

            // triangulates
            polygon_res = polygon_res
                .into_iter()
                .flat_map(|polygon| {
                    // huh
                    let reverse = polygon
                        .normal()
                        .unwrap()
                        .dot(triangle_normal.into())
                        .is_sign_negative();

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

                    let weird_overflow = h_count.saturating_sub(h).saturating_sub(1);
                    // let h = h_count - h - 1; // the texture is exported from top left while uv is bottom left
                    let material = format!("{}{}{}.bmp", texture_file_name, w, weird_overflow);

                    Triangle {
                        material,
                        vertices: vec![v0, v1, v2],
                    }
                })
                .collect::<Vec<Triangle>>()
        })
        .collect::<Vec<Triangle>>();

    smd.triangles = new_triangles;

    // split smds
    // TODO: split model, maybe too much
    let smds = maybe_split_smd(&smd);

    smds.iter().enumerate().for_each(|(idx, smd)| {
        smd.write(smd_path.with_file_name(format!("{}{}_blbh.smd", smd_file_name, idx)))
            .expect("cannot write smd file");
    });

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

        // // 270 rotation is needed.
        // qc.add_origin(
        //     options.origin.x * -1.,
        //     options.origin.y * -1.,
        //     options.origin.z * -1.,
        //     Some(270.),
        // );

        // cannot just add texture indiscriminately
        // it is possible that UV does not cover some texture
        // if that happens and we still add texrendermode for that unused texture
        // there will be "Texture too large!" error.
        let used_textures = textures_used_in_triangles(smd.triangles.as_slice());

        // add some texture flags
        used_textures.iter().for_each(|tex| {
            // always add mipmapping
            qc.add_texrendermode(tex, qc::RenderMode::NoMips);

            if options.flat_shade {
                qc.add_texrendermode(tex, qc::RenderMode::FlatShade);
            }
        });

        smds.iter().enumerate().for_each(|(idx, _smd)| {
            qc.add_body(
                format!("studio{}", idx).as_str(),
                format!("{}{}_blbh", smd_file_name, idx).as_str(),
                false,
                None,
            );
        });

        qc.add_sequence("idle", "idle", vec![]);

        let qc_path = smd_path.with_file_name(format!("{}_blbh.qc", smd_file_name));
        qc.write(qc_path.as_path())?;

        // run studiomdl
        #[cfg(target_arch = "x86_64")]
        {
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
                    const DEFAULT_STDOUT_ERROR: &str = "Cannot get stdout. This is not an error.";

                    let output = res?;
                    #[cfg(target_os = "linux")]
                    let stdout = from_utf8(&output.stdout).unwrap_or(DEFAULT_STDOUT_ERROR);

                    #[cfg(target_os = "windows")]
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    let maybe_err = stdout.find(STUDIOMDL_ERROR_PATTERN);

                    if let Some(err_index) = maybe_err {
                        let err = stdout[err_index + STUDIOMDL_ERROR_PATTERN.len()..].to_string();
                        let err_str = format!("cannot compile: {}", err.trim());

                        // it is a sin to propagate  text like this i know
                        // but i hope rust ezro cosst absraction will save me
                        let err_str = if err.contains("not found") {
                            format!("{}
If the mentioned texture file is the baked texture, make sure the texture name matches the texture file name.", err_str)
                        } else {
                            err_str
                        };

                        return Err(eyre!(err_str));
                    }
                }
                Err(_) => {
                    let err_str =
                        "No idea what happens with running studiomdl. Probably just a dream.";

                    return Err(eyre!(err_str));
                }
            };
        }

        #[cfg(target_arch = "wasm32")]
        todo!("blbh wasm32");
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
            // origin: DVec3::ZERO,
            uv_clamp_factor: BLBH_DEFAULT_UV_CLAMP_FACTOR,
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
