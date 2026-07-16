use glam::{DVec2, DVec3};
use image::{RgbaImage, imageops};
use mdl::Mdl;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use smd::{Triangle, Vertex};
use std::{
    array::from_fn,
    collections::HashMap,
    f64::consts::PI,
    path::{Path, PathBuf},
};
use studiomdl::StudioMdl;

use ndarray::prelude::*;

use common::{
    constants::MAX_GOLDSRC_MODEL_TEXTURE_COUNT,
    img_stuffs::{GoldSrcBmp, rgba8_to_8bpp},
};

#[derive(Clone)]
pub struct SkyModOptions {
    pub skybox_size: u32,
    pub texture_per_side: u32,
    pub flatshade: bool,
    pub output_name: String,
}

impl Default for SkyModOptions {
    fn default() -> Self {
        Self {
            skybox_size: 131072,
            texture_per_side: 1,
            flatshade: true,
            output_name: "skybox".to_string(),
        }
    }
}

const MIN_TEXTURE_SIZE: u32 = 512;

#[derive(Debug, Clone, thiserror::Error)]
pub enum SkymodError {
    #[error("Not all textures are square")]
    NotSquare,
    #[error("Not all textures have same dimensions. Expect {dim}x{dim}")]
    NotSameDimensions { dim: u32 },
    #[error("Failed to compile model. Please open a GitHub issue with your textures.")]
    FailCompile,
    #[error("Failed to open texture files: {paths:?}")]
    FailOpenTexture { paths: Vec<String> },
}

pub fn skymod(cubemap: [RgbaImage; 6], options: SkyModOptions) -> Result<Vec<Mdl>, SkymodError> {
    let texture0_dimensions = cubemap[0].dimensions();

    if !cubemap
        .iter()
        .any(|texture| texture.dimensions().0 == texture.dimensions().1)
    {
        return Err(SkymodError::NotSquare);
    }

    if !cubemap
        .iter()
        .any(|texture| texture.dimensions() == texture0_dimensions)
    {
        return Err(SkymodError::NotSameDimensions {
            dim: texture0_dimensions.0,
        });
    }

    let texture_per_side = options.texture_per_side;
    let texture_per_face = texture_per_side * texture_per_side;

    // ok do stuffs
    // assumptions
    // texture size is at least 512
    // if texture size is greater than 512 eg 1024 2048,
    // depending on the face count, there will be some cropping and resizing
    // cropping is when face count is more than 1
    // resize is when total amount of face count is "less" than texture size
    let min_size = texture_per_side * MIN_TEXTURE_SIZE;

    let material_name = |index, x, y| {
        format!(
            "{}{}{:02}{:02}",
            options.output_name,
            map_index_to_suffix(index),
            x,
            y
        )
    };

    let model_name = |index: usize| format!("{}{}", options.output_name, index);

    // convert textures
    let texture_lookup: HashMap<(u32, u32), GoldSrcBmp> = cubemap
        .into_par_iter()
        .enumerate()
        .flat_map(|(_texture_index, texture)| {
            let (width, _) = texture.dimensions();

            // it is best to resize first then we can crop accordingly to how many textures in a face
            let texture = if min_size == width {
                texture
            } else {
                imageops::resize(&texture, min_size, min_size, imageops::FilterType::Lanczos3)
            };

            (0..texture_per_side)
                .flat_map(|_x| {
                    (0..texture_per_side)
                        .map(|_y| {
                            let x = MIN_TEXTURE_SIZE * _x;
                            let y = MIN_TEXTURE_SIZE * _y;

                            let section = imageops::crop_imm(
                                &texture,
                                x,
                                y,
                                MIN_TEXTURE_SIZE,
                                MIN_TEXTURE_SIZE,
                            )
                            .to_image();

                            // DEBUG
                            // section
                            //     .save(
                            //         PathBuf::from("/home/khang/gchimp/examples/skybox/aaaaaaaaaa/")
                            //             .join(material_name(_texture_index as u32, _x, _y))
                            //             .with_extension("png"),
                            //     )
                            //     .unwrap();

                            let mut res = rgba8_to_8bpp(section).unwrap();
                            res.pad_palette(); // .mdl is hardcoded to have 256 colors

                            ((_x, _y), res)
                        })
                        .collect::<HashMap<(u32, u32), GoldSrcBmp>>()
                })
                .collect::<HashMap<(u32, u32), GoldSrcBmp>>()
        })
        .collect();

    // skybox size 64 means it goes from +32 to -32
    let skybox_coord = options.skybox_size as f64 / 2.;
    // size of the texture in the world, like 64k x 64k
    // TODO can move this to the loop so that all texture don't need uniform dimension side
    let texture_world_size = options.skybox_size as f64 / texture_per_side as f64;

    // write .smd, plural
    let _texture_count = texture_per_side * texture_per_side * 6;
    let model_count = (texture_lookup.len() / MAX_GOLDSRC_MODEL_TEXTURE_COUNT) + 1;

    let mut studiomdls = vec![StudioMdl::new(); model_count];

    // adding vertices to model
    for texture_index in 0..6 {
        for _y in 0..texture_per_side {
            for _x in 0..texture_per_side {
                let material_name = material_name(texture_index, _x, _y);

                // sequentially, what is the order of this texture
                // if it is over MAX_GOLDSRC_MODEL_TEXTURE_COUNT then add the quad
                // in a different smd
                let curr_texture_overall_count =
                    texture_index * texture_per_face + _y * texture_per_side + _x;

                let mdl_index =
                    curr_texture_overall_count as usize / MAX_GOLDSRC_MODEL_TEXTURE_COUNT;

                let texture_world_min_x = skybox_coord - texture_world_size * _x as f64;
                let texture_world_min_y = skybox_coord - texture_world_size * _y as f64;

                // triangle with normal vector pointing up
                // orientation is "default" where top left is 1 1 and bottom right is -1, -1
                // counter-clockwise
                // A ---- D
                // |      |
                // B ---- C
                // A has coordinate of `min`
                // C has coordinate of `max`
                let rot_mat = array![[1., 0.], [0., -1.]];

                // fix the seam, i guess?
                // zoom everything in so that the original size is 1 pixel outward diagonally for every corner
                let what: f64 = -1. / MIN_TEXTURE_SIZE as f64;

                let mut vert_a_uv = array![0. - what, 0. - what].dot(&rot_mat);
                let mut vert_b_uv = array![0. - what, 1. + what].dot(&rot_mat);
                let mut vert_c_uv = array![1. + what, 1. + what].dot(&rot_mat);
                let mut vert_d_uv = array![1. + what, 0. - what].dot(&rot_mat);

                // FIXME: dont do this
                vert_a_uv[1] += 1.0;
                vert_b_uv[1] += 1.0;
                vert_c_uv[1] += 1.0;
                vert_d_uv[1] += 1.0;

                let quad = array![
                    // A
                    [texture_world_min_x, texture_world_min_y, -skybox_coord],
                    // B
                    [
                        texture_world_min_x,
                        texture_world_min_y - texture_world_size,
                        -skybox_coord
                    ],
                    // C
                    [
                        texture_world_min_x - texture_world_size,
                        texture_world_min_y - texture_world_size,
                        -skybox_coord
                    ],
                    // D
                    [
                        texture_world_min_x - texture_world_size,
                        texture_world_min_y,
                        -skybox_coord
                    ]
                ];

                let quad = rotate_matrix_by_index_relative_to_down(texture_index, quad);
                let mut quad = quad.rows().into_iter();

                let vert_a = quad.next().unwrap();
                let vert_b = quad.next().unwrap();
                let vert_c = quad.next().unwrap();
                let vert_d = quad.next().unwrap();

                let vert_a = vert_a.as_slice().unwrap();
                let vert_b = vert_b.as_slice().unwrap();
                let vert_c = vert_c.as_slice().unwrap();
                let vert_d = vert_d.as_slice().unwrap();

                let parent = 0;

                let vert_a = Vertex {
                    parent,
                    pos: DVec3::from_slice(vert_a),
                    norm: map_index_to_norm(texture_index),
                    uv: DVec2::from_slice(vert_a_uv.as_slice().unwrap()),
                    source: None,
                };
                let vert_b = Vertex {
                    parent,
                    pos: DVec3::from_slice(vert_b),
                    norm: map_index_to_norm(texture_index),
                    uv: DVec2::from_slice(vert_b_uv.as_slice().unwrap()),
                    source: None,
                };
                let vert_c = Vertex {
                    parent,
                    pos: DVec3::from_slice(vert_c),
                    norm: map_index_to_norm(texture_index),
                    uv: DVec2::from_slice(vert_c_uv.as_slice().unwrap()),
                    source: None,
                };
                let vert_d = Vertex {
                    parent,
                    pos: DVec3::from_slice(vert_d),
                    norm: map_index_to_norm(texture_index),
                    uv: DVec2::from_slice(vert_d_uv.as_slice().unwrap()),
                    source: None,
                };

                let material = material_name.as_str();

                let tri1 = Triangle {
                    material: material.to_owned(),
                    vertices: vec![vert_a.clone(), vert_b, vert_c.clone()],
                };

                let tri2 = Triangle {
                    material: material.to_owned(),
                    vertices: vec![vert_a, vert_c, vert_d],
                };

                // add vertices
                studiomdls[mdl_index].add_triangle(tri1);
                studiomdls[mdl_index].add_triangle(tri2);

                // add materials
                let current_material = texture_lookup
                    .get(&(_x, _y))
                    .expect("enumerate through texture side length does not match lookup table");

                studiomdls[mdl_index].add_texture((
                    material_name,
                    current_material.dimensions,
                    current_material.image.clone(),
                    from_fn(|i| current_material.palette[i]),
                    mdl::TextureFlag::NOMIPS
                        | mdl::TextureFlag::FLATSHADE
                        | mdl::TextureFlag::FULLBRIGHT,
                ));
            }
        }
    }

    // compile model
    let mdls: Vec<Mdl> = studiomdls
        .into_iter()
        .enumerate()
        .flat_map(|(mdl_index, mut studiomdl)| {
            studiomdl.set_model_name(model_name(mdl_index));

            studiomdl.compile()
        })
        .collect();

    if mdls.len() != model_count {
        return Err(SkymodError::FailCompile);
    }

    Ok(mdls)
}

pub fn load_textures(texture_paths: [impl Into<String>; 6]) -> Result<[RgbaImage; 6], SkymodError> {
    let texture_paths: Vec<PathBuf> = texture_paths
        .into_iter()
        .map(|x| {
            let s: String = x.into();
            PathBuf::from(s)
        })
        .collect();
    let mut textures = Vec::with_capacity(texture_paths.len());
    let mut failures = vec![];

    for path in &texture_paths {
        match image::open(path) {
            Ok(img) => textures.push(img.to_rgba8()),
            Err(_) => failures.push(path.display().to_string()),
        }
    }

    if !failures.is_empty() {
        return Err(SkymodError::FailOpenTexture { paths: failures });
    }

    // "sort" the list
    for index in 0..texture_paths.len() {
        let path = &texture_paths[index];
        let target = map_file_name_to_index(path);

        textures.swap(index, target as usize);
    }

    Ok(from_fn(|i| textures[i].clone()))
}

pub fn map_index_to_suffix(i: u32) -> String {
    match i {
        0 => "up",
        1 => "lf",
        2 => "ft",
        3 => "rt",
        4 => "bk",
        5 => "dn",
        _ => unreachable!(),
    }
    .to_string()
}

pub fn map_file_name_to_index(p: &Path) -> u32 {
    // funny non ascii crash
    fn take_last_n_chars(s: &str, n: usize) -> &str {
        match s.char_indices().rev().nth(n - 1) {
            Some((char_idx, _)) => &s[char_idx..],
            None => s, // If s has fewer than n characters, return the whole string
        }
    }

    match take_last_n_chars(&p.file_stem().unwrap().to_string_lossy(), 2) {
        "up" => 0,
        "lf" => 1,
        "ft" => 2,
        "rt" => 3,
        "bk" => 4,
        "dn" => 5,
        _ => unreachable!(),
    }
}

fn map_index_to_norm(i: u32) -> DVec3 {
    match i {
        0 => DVec3::from_array([0., 0., -1.]),
        1 => DVec3::from_array([0., -1., 0.]),
        2 => DVec3::from_array([-1., 0., 0.]),
        3 => DVec3::from_array([0., 1., 0.]),
        4 => DVec3::from_array([1., 0., 0.]),
        5 => DVec3::from_array([0., 0., 1.]),
        _ => unreachable!(),
    }
}

fn rotx_matrix(theta: f64) -> ArrayBase<ndarray::OwnedRepr<f64>, Dim<[usize; 2]>> {
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();

    array![
        [1., 0., 0.],
        [0., cos_theta, -sin_theta],
        [0., sin_theta, cos_theta]
    ]
}

fn roty_matrix(theta: f64) -> ArrayBase<ndarray::OwnedRepr<f64>, Dim<[usize; 2]>> {
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();

    array![
        [cos_theta, 0., sin_theta],
        [0., 1., 0.],
        [-sin_theta, 0., cos_theta]
    ]
}

fn rotz_matrix(theta: f64) -> ArrayBase<ndarray::OwnedRepr<f64>, Dim<[usize; 2]>> {
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();

    array![
        [cos_theta, -sin_theta, 0.],
        [sin_theta, cos_theta, 0.],
        [0., 0., 1.],
    ]
}

fn rotate_matrix_by_index_relative_to_down(
    index: u32,
    vert: ArrayBase<ndarray::OwnedRepr<f64>, Dim<[usize; 2]>>,
) -> ArrayBase<ndarray::OwnedRepr<f64>, Dim<[usize; 2]>> {
    let theta = PI / 2.;
    let horizontal_flip = array![[-1., 0., 0.,], [0., 1., 0.], [0., 0., 1.]];
    let vertical_flip = array![[1., 0., 0.,], [0., -1., 0.], [0., 0., 1.]];

    //          y
    //          ^
    //          |
    //          |
    // x<-------+
    match index {
        // up
        0 => vert
        .dot(&horizontal_flip)
        .dot(&rotx_matrix(-theta))
        .dot(&rotx_matrix(-theta))
        .dot(&rotz_matrix(theta))
        .dot(&rotz_matrix(theta))
        ,
        // left, wow axis is like normal math
        1 => vert
        .dot(&horizontal_flip)
        .dot(&rotx_matrix(-theta)),
        // front
        2 => vert.dot(&horizontal_flip)
        .dot(&roty_matrix(-theta))
        .dot(&rotx_matrix(-theta)),
        // right
        3 => vert.dot(&vertical_flip).dot(&rotx_matrix(theta)),
        // back
        4 => vert
        .dot(&horizontal_flip)
            .dot(&roty_matrix(theta))
            .dot(&rotx_matrix(-theta))
            ,
        // down
        5 => vert.dot(&vertical_flip)
        ,
        _ => unreachable!(),
    }
}

#[allow(dead_code)]
// fix output from https://matheowis.github.io/HDRI-to-CubeMap/ individual texture save option
fn fix_matheowis_hdri_to_cubemap_rotation(folder: impl Into<PathBuf> + AsRef<Path>) {
    let folder = folder.as_ref();

    // pz: up
    std::fs::copy(folder.join("pz.png"), folder.join("outup.png")).unwrap();

    // nz: down + 180*
    image::open(folder.join("nz.png"))
        .unwrap()
        .rotate180()
        .save(folder.join("outdn.png"))
        .unwrap();

    // py: left + 180*
    image::open(folder.join("py.png"))
        .unwrap()
        .rotate180()
        .save(folder.join("outlf.png"))
        .unwrap();

    // ny: right
    std::fs::rename(folder.join("ny.png"), folder.join("outrt.png")).unwrap();

    // px: front -90 *(like the normal coordinate)
    image::open(folder.join("px.png"))
        .unwrap()
        .rotate90() // -90 degrees is the same as +270 degrees rotation
        .save(folder.join("outft.png"))
        .unwrap();

    // nx: back + 90*
    image::open(folder.join("nx.png"))
        .unwrap()
        .rotate270()
        .save(folder.join("outbk.png"))
        .unwrap();
}

#[cfg(test)]
mod test {
    use common::img_stuffs::hdri_to_cubemap;

    use super::*;

    #[test]
    #[ignore]
    fn run() {
        let gchimp_modules = env!("CARGO_MANIFEST_DIR");
        let paths = [
            format!("{gchimp_modules}/../examples/skybox/test2bk.png"),
            format!("{gchimp_modules}/../examples/skybox/test2dn.png"),
            format!("{gchimp_modules}/../examples/skybox/test2ft.png"),
            format!("{gchimp_modules}/../examples/skybox/test2lf.png"),
            format!("{gchimp_modules}/../examples/skybox/test2rt.png"),
            format!("{gchimp_modules}/../examples/skybox/test2up.png"),
        ];

        let texture_per_side = 1;

        let options = SkyModOptions {
            skybox_size: 1024,
            texture_per_side,
            flatshade: true,
            output_name: "nineface".into(),
        };

        let textures = load_textures(paths).unwrap();

        let mdls = skymod(textures, options).unwrap();

        assert_eq!(mdls.len(), 1);

        let mdl = mdls[0].clone();

        let write_bytes = mdl.write_to_bytes();

        // parse test
        let read_mdl = mdl::Mdl::open_from_bytes(&write_bytes).unwrap();

        assert_eq!(read_mdl.bodyparts.len(), 1);
        assert_eq!(read_mdl.bodyparts[0].models.len(), 1);

        {
            let model = &read_mdl.bodyparts[0].models[0];
            assert_eq!(
                model.header.num_mesh as u32,
                texture_per_side * texture_per_side * 6
            );

            // println!("{:?}", model.header);
        }

        // println!("{:?}", read_mdl.sequences[0]);

        mdl.write_to_file(format!(
            "{gchimp_modules}/../examples/skybox/refactored_test2.mdl"
        ))
        .unwrap();
    }

    #[test]
    #[ignore]
    fn _run2() {
        let gchimp_modules = env!("CARGO_MANIFEST_DIR");
        let paths = [
            format!("{gchimp_modules}/..examples/skybox/cyberwaveRT.png"),
            format!("{gchimp_modules}/..examples/skybox/cyberwaveBK.png"),
            format!("{gchimp_modules}/..examples/skybox/cyberwaveLF.png"),
            format!("{gchimp_modules}/..examples/skybox/cyberwaveDN.png"),
            format!("{gchimp_modules}/..examples/skybox/cyberwaveUP.png"),
            format!("{gchimp_modules}/..examples/skybox/cyberwaveFT.png"),
        ];

        let options = SkyModOptions {
            skybox_size: Default::default(),
            texture_per_side: 1,
            flatshade: true,
            output_name: "gchimp_lets_go".into(),
        };

        let textures = load_textures(paths).unwrap();

        let mdls = skymod(textures, options).unwrap();
    }

    #[test]
    #[ignore]
    fn fix_skybox_rotation() {
        let folder = "/home/khang/map/arte_dela/skybox/";

        fix_matheowis_hdri_to_cubemap_rotation(folder);
    }

    #[test]
    #[ignore]
    fn hdri_to_cubemap_test() {
        let hdri = "/home/khang/Downloads/suburban_garden_4k.exr";
        let img = image::open(hdri).unwrap();

        let cubemap = hdri_to_cubemap(&img.into(), 512, 12.);
        for (suffix, x) in cubemap {
            x.save(format!(
                "/home/khang/Downloads/suburban_garden_4k_{}.png",
                suffix
            ))
            .unwrap();
        }
    }
}
