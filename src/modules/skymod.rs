use eyre::eyre;
use image::{imageops, DynamicImage, RgbaImage};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};
use smd::Smd;
use std::path::PathBuf;

use crate::utils::img_stuffs::{rgba8_to_8bpp, write_8bpp};

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

        let mut textures = self
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

        let sqrt = (self.options.texture_per_face as f32).sqrt().round() as u32;
        if sqrt * sqrt != self.options.texture_per_face {
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

        let min_size = self.options.texture_per_face * MIN_TEXTURE_SIZE;

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

                for _y in 0..sqrt {
                    for _x in 0..sqrt {
                        let x = MIN_TEXTURE_SIZE * _x;
                        let y = MIN_TEXTURE_SIZE * _y;

                        let section =
                            imageops::crop_imm(&texture, x, y, MIN_TEXTURE_SIZE, MIN_TEXTURE_SIZE)
                                .to_image();
                        let (img, palette) = rgba8_to_8bpp(section).unwrap();

                        let texture_file_name = format!(
                            "{}{}{}{}",
                            self.output_name,
                            map_index_to_suffix(texture_index as u32),
                            _y,
                            _x
                        );

                        write_8bpp(
                            &img,
                            &palette,
                            (MIN_TEXTURE_SIZE, MIN_TEXTURE_SIZE),
                            texture_file_name.as_str(),
                        )
                        .unwrap();
                    }
                }
            });

        let new_smd = Smd::new_basic();

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
            .studiomdl("what")
            .wineprefix("what")
            .output_name("please")
            .texture_per_face(4);

        let res = builder.work();

        assert!(res.is_ok());
    }
}
