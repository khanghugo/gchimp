use eyre::eyre;
use glam::DVec3;
use image::{imageops, RgbaImage};
use qc::Qc;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use smd::{extras::SmdExtras, Smd};
use std::{f32::consts::PI, ops::Mul, path::PathBuf};

use ndarray::prelude::*;

use crate::utils::{
    img_stuffs::{rgba8_to_8bpp, write_8bpp},
    run_bin::run_studiomdl,
};

pub struct SkyModOptions {
    skybox_size: u32,
    texture_per_face: u32,
}

impl Default for SkyModOptions {
    fn default() -> Self {
        Self {
            skybox_size: 131072,
            texture_per_face: 1,
        }
    }
}

static MIN_TEXTURE_SIZE: u32 = 512;

pub struct SkyModBuilder {
    // order is: 0 up 1 left 2 front 3 right 4 back 5 down
    textures: Vec<String>,
    options: SkyModOptions,
    output_name: String,
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
            output_name: "".to_string(),
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

    pub fn wineprefix(&mut self, a: &str) -> &mut Self {
        self.wineprefix = Some(a.into());
        self
    }

    pub fn output_name(&mut self, a: &str) -> &mut Self {
        self.output_name = a.to_string();
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
            return Err(eyre!("Cannot parse all texture files"));
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
            } else {
                if width % MIN_TEXTURE_SIZE != 0 {
                    return Err(eyre!(
                        "Does not support textures with size not multiple of {} ({}): {}",
                        MIN_TEXTURE_SIZE,
                        width,
                        self.textures[index]
                    ));
                }
            }
        }

        let side = textures[0].dimensions().0;
        let same_dimension_all_texture = textures
            .iter()
            .fold(true, |acc, e| e.dimensions().0 == side && true);
        if !same_dimension_all_texture {
            return Err(eyre!(
                "Does not support individual texture with different dimension from another"
            ));
        }

        let texture_per_side = (self.options.texture_per_face as f32).sqrt().round() as u32;
        if texture_per_side * texture_per_side != self.options.texture_per_face {
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

        // writes .bmp
        textures
            .into_par_iter()
            .enumerate()
            .for_each(|(texture_index, texture)| {
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
                            self.output_name,
                            map_index_to_suffix(texture_index as u32),
                            _y,
                            _x
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
        let skybox_coord = (self.options.skybox_size / 2) as f32;
        // size of the texture in the world, like 64k x 64k
        let texture_world_size = self.options.skybox_size as f32 / texture_per_side as f32;

        // write .smd, plural
        let mut new_smd = Smd::new_basic();

        for texture_index in 0..6 {
            for _y in 0..texture_per_side {
                for _x in 0..texture_per_side {
                    let texture_file_name = format!(
                        "{}{}{}{}.bmp",
                        self.output_name,
                        map_index_to_suffix(texture_index as u32),
                        _y,
                        _x
                    );

                    let texture_world_min_x = skybox_coord - texture_world_size * _x as f32;
                    let texture_world_min_y = skybox_coord - texture_world_size * _y as f32;

                    // we will always start the texture with down alignment
                    // then we rotate the coordinate
                    // start top left -> +x +y
                    // z here is negative because we are down alignment
                    let min =
                        ndarray::arr2(&[[texture_world_min_x, texture_world_min_y, -skybox_coord]]);
                    let max = ndarray::arr2(&[[
                        texture_world_min_x - texture_world_size,
                        texture_world_min_y - texture_world_size,
                        -skybox_coord,
                    ]]);

                    let rot_matrix_for_side = rotating_matrix_relative_from_down(texture_index);

                    let min = min.dot(&rot_matrix_for_side);
                    let max = max.dot(&rot_matrix_for_side);

                    // do stuffs over f64 cuz smd wants it
                    let min = min
                        .into_iter()
                        .map(|what| (what as f64).round())
                        .collect::<Vec<f64>>();
                    let max = max
                        .into_iter()
                        .map(|what| (what as f64).round())
                        .collect::<Vec<f64>>();

                    let norm = map_index_to_norm(texture_index);

                    let new_triangles = Smd::square(
                        texture_file_name.as_str(),
                        min.as_slice(),
                        max.as_slice(),
                        &[norm[0], norm[1], norm[2]],
                    );

                    new_triangles.into_iter().for_each(|tri| {
                        new_smd.add_triangle(tri);
                    });
                }
            }
        }

        new_smd.write(
            root_path
                .join(format!("{}.smd", self.output_name))
                .to_str()
                .unwrap(),
        )?;

        // idle sequence to be compliant
        let idle_smd = Smd::new_basic();
        idle_smd.write(root_path.join("idle.smd").to_str().unwrap())?;

        // write qc
        let mut qc = Qc::new_basic();

        let model_name = first_texture_path
            .with_file_name(&self.output_name)
            .with_extension("mdl");

        qc.add_model_name(model_name.to_str().unwrap());
        qc.add_cd(root_path.to_str().unwrap());
        qc.add_cd_texture(root_path.to_str().unwrap());

        qc.add_body("studio0", &self.output_name, false, None);
        qc.add_sequence("idle", "idle", vec![]);

        let qc_path = root_path.join(format!("{}.qc", self.output_name));

        qc.write(qc_path.to_str().unwrap())?;

        // run studiomdl
        let handle = run_studiomdl(
            qc_path.as_path(),
            self.studiomdl.as_ref().unwrap(),
            self.wineprefix.as_ref().unwrap(),
        );

        let _ = handle.join().unwrap()?;

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

fn rotx_matrix(theta: f32) -> ArrayBase<ndarray::OwnedRepr<f32>, Dim<[usize; 2]>> {
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();

    array![
        [1., 0., 0.],
        [0., cos_theta, -sin_theta],
        [0., sin_theta, cos_theta]
    ]
}

fn roty_matrix(theta: f32) -> ArrayBase<ndarray::OwnedRepr<f32>, Dim<[usize; 2]>> {
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();

    array![
        [cos_theta, 0., sin_theta],
        [0., 1., 0.],
        [-sin_theta, 0., cos_theta]
    ]
}

fn rotz_matrix(theta: f32) -> ArrayBase<ndarray::OwnedRepr<f32>, Dim<[usize; 2]>> {
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();

    array![
        [cos_theta, -sin_theta, 0.],
        [sin_theta, cos_theta, 0.],
        [0., 0., 1.],
    ]
}

fn rotating_matrix_relative_from_front(
    i: u32,
) -> ArrayBase<ndarray::OwnedRepr<f32>, Dim<[usize; 2]>> {
    let id = array![[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

    let theta = PI / 2.;

    match i {
        // up
        0 => roty_matrix(-theta),
        // left
        1 => rotz_matrix(theta),
        // front
        2 => id,
        // right
        3 => rotz_matrix(-theta),
        // back
        4 => rotz_matrix(theta).mul(rotz_matrix(theta)),
        // down
        5 => roty_matrix(theta),
        _ => unreachable!(),
    }
}

fn rotating_matrix_relative_from_down(
    i: u32,
) -> ArrayBase<ndarray::OwnedRepr<f32>, Dim<[usize; 2]>> {
    let id = array![[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

    let theta = PI / 2.;

    match i {
        // up
        0 => roty_matrix(theta).mul(roty_matrix(theta)),
        // left
        1 => rotx_matrix(theta),
        // front
        2 => roty_matrix(-theta),
        // right
        3 => rotx_matrix(-theta),
        // back
        4 => roty_matrix(theta),
        // down
        5 => id,
        _ => unreachable!(),
    }
}

mod test {
    use super::*;

    #[test]
    fn run() {
        let mut binding = SkyModBuilder::new();
        let builder = binding
            .bk("examples/skybox/sky_cloudybk.png")
            .dn("examples/skybox/sky_cloudydn.png")
            .ft("examples/skybox/sky_cloudyft.png")
            .lf("examples/skybox/sky_cloudylf.png")
            .rt("examples/skybox/sky_cloudyrt.png")
            .up("examples/skybox/sky_cloudyup.png")
            .studiomdl("/home/khang/map2prop-rs/dist/studiomdl.exe")
            .wineprefix("/home/khang/.local/share/wineprefixes/wine32/")
            .output_name("please")
            .skybox_size(512)
            .texture_per_face(1);

        let res = builder.work();

        assert!(res.is_ok());
    }
}
