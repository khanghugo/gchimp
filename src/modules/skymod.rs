use eyre::eyre;
use glam::{DVec2, DVec3};
use image::{imageops, RgbaImage};
use qc::Qc;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use smd::{Smd, Triangle, Vertex};
use std::{f64::consts::PI, path::PathBuf, str::from_utf8};

use ndarray::prelude::*;

use crate::utils::{
    constants::{MAX_GOLDSRC_MODEL_TEXTURE_COUNT, STUDIOMDL_ERROR_PATTERN},
    img_stuffs::{rgba8_to_8bpp, write_8bpp},
    run_bin::run_studiomdl,
};

#[derive(Clone)]
pub struct SkyModOptions {
    pub skybox_size: u32,
    pub texture_per_face: u32,
    pub convert_texture: bool,
    pub flatshade: bool,
    pub output_name: String,
}

impl Default for SkyModOptions {
    fn default() -> Self {
        Self {
            skybox_size: 131072,
            texture_per_face: 1,
            convert_texture: true,
            flatshade: true,
            output_name: "skybox".to_string(),
        }
    }
}

static MIN_TEXTURE_SIZE: u32 = 512;

pub struct SkyModBuilder {
    // order is: 0 up 1 left 2 front 3 right 4 back 5 down
    textures: Vec<String>,
    options: SkyModOptions,
    studiomdl: Option<PathBuf>,
    wineprefix: Option<String>,
}

impl Default for SkyModBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SkyModBuilder {
    pub fn new() -> Self {
        Self {
            textures: vec![String::new(); 6],
            options: SkyModOptions::default(),
            studiomdl: None,
            wineprefix: None,
        }
    }

    pub fn up(&mut self, a: &str) -> &mut Self {
        a.clone_into(&mut self.textures[0]);
        self
    }

    pub fn lf(&mut self, a: &str) -> &mut Self {
        a.clone_into(&mut self.textures[1]);
        self
    }

    pub fn ft(&mut self, a: &str) -> &mut Self {
        a.clone_into(&mut self.textures[2]);
        self
    }

    pub fn rt(&mut self, a: &str) -> &mut Self {
        a.clone_into(&mut self.textures[3]);
        self
    }

    pub fn bk(&mut self, a: &str) -> &mut Self {
        a.clone_into(&mut self.textures[4]);
        self
    }

    pub fn dn(&mut self, a: &str) -> &mut Self {
        a.clone_into(&mut self.textures[5]);
        self
    }

    pub fn studiomdl(&mut self, a: &str) -> &mut Self {
        self.studiomdl = Some(a.into());
        self
    }

    pub fn wineprefix(&mut self, a: Option<String>) -> &mut Self {
        self.wineprefix = a;
        self
    }

    pub fn output_name(&mut self, a: &str) -> &mut Self {
        self.options.output_name = a.to_string();
        self
    }

    pub fn texture_per_face(&mut self, a: u32) -> &mut Self {
        self.options.texture_per_face = a;
        self
    }

    pub fn skybox_size(&mut self, a: u32) -> &mut Self {
        self.options.skybox_size = a;
        self
    }

    pub fn convert_texture(&mut self, a: bool) -> &mut Self {
        self.options.convert_texture = a;
        self
    }

    pub fn flat_shade(&mut self, a: bool) -> &mut Self {
        self.options.flatshade = a;
        self
    }

    pub fn work(&self) -> eyre::Result<()> {
        // check stuffs
        for i in 0..6 {
            if self.textures[i].is_empty() {
                return Err(eyre!("Empty texture."));
            }
        }

        if self.studiomdl.is_none() {
            return Err(eyre!("No studiomdl.exe supplied"));
        }

        #[cfg(target_os = "linux")]
        if self.wineprefix.is_none() {
            return Err(eyre!("No WINEPREFIX supplied"));
        }

        let textures = self
            .textures
            .iter()
            .filter_map(|path| image::open(path).ok())
            .map(|img| img.into_rgba8())
            .collect::<Vec<RgbaImage>>();

        if textures.len() != 6 {
            return Err(eyre!(
                "Cannot parse all texture files ({}/6)",
                textures.len()
            ));
        }

        for (index, texture) in textures.iter().enumerate() {
            let (width, height) = texture.dimensions();

            if width != height {
                return Err(eyre!(
                    "Does not support textures with mismatched size {}x{}: {}",
                    width,
                    height,
                    self.textures[index]
                ));
            } else if width % MIN_TEXTURE_SIZE != 0 {
                return Err(eyre!(
                    "Does not support textures with size not multiple of {} ({}): {}",
                    MIN_TEXTURE_SIZE,
                    width,
                    self.textures[index]
                ));
            }
        }

        let side = textures[0].dimensions().0;
        let same_dimension_all_texture = textures
            .iter()
            .fold(true, |acc, e| e.dimensions().0 == side && acc);
        if !same_dimension_all_texture {
            return Err(eyre!(
                "Does not support individual texture with different dimension from another"
            ));
        }

        let texture_per_side = (self.options.texture_per_face as f32).sqrt().floor() as u32;
        if texture_per_side * texture_per_side != self.options.texture_per_face
            || self.options.texture_per_face == 0
        {
            return Err(eyre!(
                "Chosen texture per face is not a valid number. Use n^2."
            ));
        }

        // ok do stuffs
        // assumptions
        // texture size is at least 512
        // if texture size is greater than 512 eg 1024 2048,
        // depending on the face count, there will be some cropping and resizing
        // cropping is when face count is more than 1
        // resize is when total amount of face count is "less" than texture size

        let first_texture_path = PathBuf::from(&self.textures[0]);
        let root_path = first_texture_path.parent().unwrap();

        let min_size = texture_per_side * MIN_TEXTURE_SIZE;

        // // writes .bmp
        textures
            .into_par_iter()
            .enumerate()
            .for_each(|(texture_index, texture)| {
                if !self.options.convert_texture {
                    return;
                }

                let (width, _) = texture.dimensions();

                // it is best to resize first then we can crop accordingly to how many textures in a face
                let texture = if min_size == width {
                    texture
                } else {
                    imageops::resize(&texture, min_size, min_size, imageops::FilterType::Lanczos3)
                };

                for _y in 0..texture_per_side {
                    for _x in 0..texture_per_side {
                        let x = MIN_TEXTURE_SIZE * _x;
                        let y = MIN_TEXTURE_SIZE * _y;

                        let texture_file_name = format!(
                            "{}{}{}{}.bmp",
                            self.options.output_name,
                            map_index_to_suffix(texture_index as u32),
                            _x,
                            _y
                        );

                        let section =
                            imageops::crop_imm(&texture, x, y, MIN_TEXTURE_SIZE, MIN_TEXTURE_SIZE)
                                .to_image();
                        let (img, palette) = rgba8_to_8bpp(section).unwrap();

                        write_8bpp(
                            &img,
                            &palette,
                            (MIN_TEXTURE_SIZE, MIN_TEXTURE_SIZE),
                            root_path.join(texture_file_name).as_path(),
                        )
                        .unwrap();
                    }
                }
            });

        // skybox size 64 means it goes from +32 to -32
        let skybox_coord = self.options.skybox_size as f64 / 2.;
        // size of the texture in the world, like 64k x 64k
        let texture_world_size = self.options.skybox_size as f64 / texture_per_side as f64;

        // write .smd, plural
        let texture_count = self.options.texture_per_face * 6;
        let model_count = texture_count / MAX_GOLDSRC_MODEL_TEXTURE_COUNT + 1;

        let mut new_smds = vec![Smd::new_basic(); model_count as usize];

        for texture_index in 0..6 {
            for _y in 0..texture_per_side {
                for _x in 0..texture_per_side {
                    let texture_file_name = format!(
                        "{}{}{}{}.bmp",
                        self.options.output_name,
                        map_index_to_suffix(texture_index),
                        _x,
                        _y
                    );

                    // sequentially, what is the order of this texture
                    // if it is over MAX_GOLDSRC_MODEL_TEXTURE_COUNT then add the quad
                    // in a different smd
                    let curr_texture_overall_count =
                        texture_index * self.options.texture_per_face + _y * texture_per_side + _x;

                    let new_smd_index =
                        (curr_texture_overall_count / MAX_GOLDSRC_MODEL_TEXTURE_COUNT) as usize;

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

                    let vert_a_uv = array![0. - what, 0. - what].dot(&rot_mat);
                    let vert_b_uv = array![0. - what, 1. + what].dot(&rot_mat);
                    let vert_c_uv = array![1. + what, 1. + what].dot(&rot_mat);
                    let vert_d_uv = array![1. + what, 0. - what].dot(&rot_mat);

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

                    let material = texture_file_name.as_str();

                    let tri1 = Triangle {
                        material: material.to_owned(),
                        vertices: vec![vert_a.clone(), vert_b, vert_c.clone()],
                    };

                    let tri2 = Triangle {
                        material: material.to_owned(),
                        vertices: vec![vert_a, vert_c, vert_d],
                    };

                    new_smds[new_smd_index].add_triangle(tri1);
                    new_smds[new_smd_index].add_triangle(tri2);
                }
            }
        }

        for (smd_index, new_smd) in new_smds.into_iter().enumerate() {
            new_smd.write(
                root_path
                    .join(format!("{}{}.smd", self.options.output_name, smd_index))
                    .to_str()
                    .unwrap(),
            )?;
        }

        // can reuse idle sequence for multiple smds
        // idle sequence to be compliant
        let idle_smd = Smd::new_basic();
        idle_smd.write(root_path.join("idle.smd").to_str().unwrap())?;

        // TODO dont add 0 at the end for 1 model
        for model_index in 0..model_count {
            // write qc
            let mut qc = Qc::new_basic();

            let model_name = first_texture_path
                .with_file_name(format!("{}{}", &self.options.output_name, model_index))
                .with_extension("mdl");

            qc.add_model_name(model_name.to_str().unwrap());
            qc.add_cd(root_path.to_str().unwrap());
            qc.add_cd_texture(root_path.to_str().unwrap());

            if self.options.flatshade {
                for texture_index in 0..6 {
                    for _y in 0..texture_per_side {
                        for _x in 0..texture_per_side {
                            let curr_texture_overall_count = texture_index
                                * self.options.texture_per_face
                                + _y * texture_per_side
                                + _x;
                            let new_qc_index = (curr_texture_overall_count
                                / MAX_GOLDSRC_MODEL_TEXTURE_COUNT)
                                as usize;

                            if new_qc_index != model_index as usize {
                                continue;
                            }

                            let texture_file_name = format!(
                                "{}{}{}{}.bmp",
                                self.options.output_name,
                                map_index_to_suffix(texture_index),
                                _x,
                                _y
                            );

                            qc.add_texrendermode(&texture_file_name, qc::RenderMode::FlatShade);
                        }
                    }
                }
            }

            qc.add_body(
                "studio0",
                format!("{}{}", self.options.output_name, model_index).as_str(),
                false,
                None,
            );
            qc.add_sequence("idle", "idle", vec![]);

            let qc_path = root_path.join(format!("{}{}.qc", self.options.output_name, model_index));

            qc.write(qc_path.to_str().unwrap())?;

            // run studiomdl
            #[cfg(target_os = "windows")]
            let handle = run_studiomdl(qc_path.as_path(), self.studiomdl.as_ref().unwrap());

            #[cfg(target_os = "linux")]
            let handle = run_studiomdl(
                qc_path.as_path(),
                self.studiomdl.as_ref().unwrap(),
                self.wineprefix.as_ref().unwrap(),
            );

            match handle.join() {
                Ok(res) => {
                    let output = res?;
                    let stdout = from_utf8(&output.stdout).unwrap();

                    let maybe_err = stdout.find(STUDIOMDL_ERROR_PATTERN);

                    if let Some(err_index) = maybe_err {
                        let err = stdout[err_index + STUDIOMDL_ERROR_PATTERN.len()..].to_string();
                        let err_str = format!("Cannot compile: {}", err.trim());
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

        Ok(())
    }
}

fn map_index_to_suffix(i: u32) -> String {
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

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn run() {
        let mut binding = SkyModBuilder::new();
        let builder = binding
            .bk("examples/skybox/test2bk.png")
            .dn("examples/skybox/test2dn.png")
            .ft("examples/skybox/test2ft.png")
            .lf("examples/skybox/test2lf.png")
            .rt("examples/skybox/test2rt.png")
            .up("examples/skybox/test2up.png")
            .studiomdl("/home/khang/gchimp/dist/studiomdl.exe")
            .wineprefix(Some(
                "/home/khang/.local/share/wineprefixes/wine32/".to_owned(),
            ))
            .output_name("nineface")
            .skybox_size(512)
            .texture_per_face(9)
            .convert_texture(false);

        let res = builder.work();

        println!("{:?}", res);

        assert!(res.is_ok());
    }

    #[test]
    fn run2() {
        let mut binding = SkyModBuilder::new();
        let builder = binding
            .bk("examples/skybox/cyberwaveBK.png")
            .dn("examples/skybox/cyberwaveDN.png")
            .ft("examples/skybox/cyberwaveFT.png")
            .lf("examples/skybox/cyberwaveLF.png")
            .rt("examples/skybox/cyberwaveRT.png")
            .up("examples/skybox/cyberwaveUP.png")
            .studiomdl("/home/khang/gchimp/dist/studiomdl.exe")
            .wineprefix(Some(
                "/home/khang/.local/share/wineprefixes/wine32/".to_owned(),
            ))
            .output_name("gchimp_lets_go")
            .skybox_size(2_u32.pow(17))
            .texture_per_face(1)
            .convert_texture(true);

        let res = builder.work();

        println!("{:?}", res);

        assert!(res.is_ok());
    }
}
