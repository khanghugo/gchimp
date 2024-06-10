use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use eyre::eyre;
use image::RgbaImage;

use rayon::prelude::*;

use crate::utils::img_stuffs::{
    eight_bpp_transparent_img, rgba8_to_8bpp, tile_and_resize, write_8bpp_to_file, GoldSrcBmp,
};

pub struct TexTileBuilder {
    items: Vec<PathBuf>,
    options: TexTileOptions,
    sync: Option<TexTileSync>,
}

pub struct TexTileOptions {
    pub extensions: Vec<String>,
    pub is_tiling: bool,
    /// Multiply the dimension by this number
    pub tiling_scalar: u32,
    pub is_transparent: bool,
    /// \[0, 1\]
    pub transparent_threshold: f32,
    /// Prepends "{" if transparent
    ///
    /// Appends "_<scalar>" if tiling
    pub change_name: bool,
}

impl Default for TexTileOptions {
    fn default() -> Self {
        Self {
            extensions: vec!["png".to_string(), "jpg".to_string(), "jpeg".to_string()],
            is_tiling: true,
            tiling_scalar: 2,
            is_transparent: false,
            transparent_threshold: 0.75,
            change_name: true,
        }
    }
}

impl TexTileOptions {
    pub fn check_item(&self, item: &Path) -> eyre::Result<()> {
        if !item.exists() {
            return Err(eyre!("Item {} does not exist", item.display()));
        }

        if item.is_file() {
            if let Some(extension) = item.extension() {
                if !self
                    .extensions
                    .contains(&extension.to_str().unwrap().to_owned())
                {
                    return Err(eyre!(
                        "Item {} does not have the qualified extension",
                        item.display()
                    ));
                }
            } else {
                return Err(eyre!(
                    "Item {} is a file and does not have any extension.",
                    item.display()
                ));
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct TexTileSync {
    status: Arc<Mutex<String>>,
    done: Arc<Mutex<bool>>,
    // i tried it and the callback pattern is very complicated to implement :()
    callback: fn(),
}

impl TexTileSync {
    pub fn status(&self) -> &Arc<Mutex<String>> {
        &self.status
    }

    pub fn done(&self) -> &Arc<Mutex<bool>> {
        &self.done
    }

    pub fn set_callback(&mut self, callback: fn()) -> &mut Self {
        self.callback = callback;
        self
    }
}

impl Default for TexTileSync {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(String::from("Idle"))),
            done: Arc::new(Mutex::new(true)),
            callback: || {},
        }
    }
}

impl TexTileBuilder {
    pub fn new(items: Vec<PathBuf>) -> Self {
        Self {
            items,
            options: TexTileOptions::default(),
            sync: None,
        }
    }

    fn check_items(&self) -> eyre::Result<()> {
        for item in self.items.iter() {
            let check = self.options.check_item(item);

            check?;
        }

        Ok(())
    }

    pub fn sync(&mut self, sync: TexTileSync) -> &mut Self {
        self.sync = Some(sync);
        self
    }

    fn log(&mut self, s: impl AsRef<str> + Into<String>) {
        println!("{}", s.as_ref());

        if let Some(sync) = &self.sync {
            let mut status = sync.status.lock().unwrap();

            *status = s.into();
            (sync.callback)();
        }
    }

    pub fn extension(&mut self, a: &[String]) -> &mut Self {
        self.options.extensions = a.to_vec();
        self
    }

    pub fn tiling(&mut self, a: bool) -> &mut Self {
        self.options.is_tiling = a;
        self
    }

    pub fn tiling_scalar(&mut self, a: u32) -> &mut Self {
        self.options.tiling_scalar = a;
        self
    }

    pub fn transparent(&mut self, a: bool) -> &mut Self {
        self.options.is_transparent = a;
        self
    }

    /// \[0, 1\]
    pub fn transparent_threshold(&mut self, a: f32) -> &mut Self {
        self.options.transparent_threshold = a;
        self
    }

    /// Prepends "{" if transparent
    ///
    /// Appends "_<scalar>" if tiling
    pub fn change_name(&mut self, a: bool) -> &mut Self {
        self.options.change_name = a;
        self
    }

    pub fn work(&mut self) -> eyre::Result<()> {
        // transparent shoudl be the last step
        // the reason is that transparent pixel could be interpolated when tiling or scaled down
        // that leads to it not being transparent anymore
        if let Some(sync) = &self.sync {
            *sync.status.lock().unwrap() = "Running".to_string();
        }

        self.check_items()?;

        let mut work_items: Vec<PathBuf> = vec![];

        // because some items are folders, we need to find all of qualified image files from those folders
        for item in self.items.clone() {
            if item.is_file() {
                work_items.push(item)
            } else {
                let huh = fs::read_dir(item).unwrap();
                let paths = huh
                    .filter_map(|read_dir| read_dir.ok())
                    .map(|entry| entry.path());

                paths
                    .filter(|path| path.is_file() && self.options.check_item(path).is_ok())
                    .for_each(|path| work_items.push(path));
            }
        }

        // load the images into rgba8
        let rgba_images: Vec<eyre::Result<(PathBuf, RgbaImage)>> = work_items
            .into_par_iter()
            .map(|work_item| {
                let new_img = image::open(&work_item);

                if new_img.is_err() {
                    let log = format!(
                        "Cannot open image {}: {}",
                        work_item.display(),
                        new_img.unwrap_err()
                    );

                    return Err(eyre!(log));
                }

                Ok((work_item, new_img.unwrap().into_rgba8()))
            })
            .collect();

        if let Some(Err(err)) = rgba_images.iter().find(|res| res.is_err()) {
            let err_str = err.to_string();

            self.log(&err_str);

            return Err(eyre!(err_str));
        }

        let mut rgba_images: Vec<(PathBuf, RgbaImage)> =
            rgba_images.into_iter().map(|res| res.unwrap()).collect();

        if self.options.is_tiling {
            rgba_images.par_iter_mut().for_each(|(_, img)| {
                *img = tile_and_resize(img, self.options.tiling_scalar);
            });
        }

        // with all of the things processing rgba8 done, now we convert them to 8bpp for other steps
        let eight_bpps = rgba_images
            .into_par_iter()
            .map(|(path, img)| {
                let new_img = rgba8_to_8bpp(img);

                if new_img.is_err() {
                    let log = format!(
                        "Cannot convert {} to 8bpp: {}",
                        path.display(),
                        new_img.unwrap_err()
                    );

                    return Err(eyre!(log));
                }

                Ok((path, new_img.unwrap()))
            })
            .collect::<Vec<eyre::Result<(PathBuf, GoldSrcBmp)>>>();

        if let Some(Err(err)) = eight_bpps.iter().find(|res| res.is_err()) {
            let err_str = err.to_string();

            self.log(&err_str);

            return Err(eyre!(err_str));
        }

        let mut eight_bpps: Vec<(PathBuf, GoldSrcBmp)> =
            eight_bpps.into_iter().map(|res| res.unwrap()).collect();

        if self.options.is_transparent {
            eight_bpps.par_iter_mut().for_each(
                |(
                    _,
                    GoldSrcBmp {
                        img,
                        palette,
                        dimension: _,
                    },
                )| {
                    let (new_img, new_palette) =
                        eight_bpp_transparent_img(img, palette, self.options.transparent_threshold);

                    *img = new_img;
                    *palette = (*new_palette).to_vec();
                },
            );
        }

        let write_res = eight_bpps
            .into_par_iter()
            .map(
                |(
                    path,
                    GoldSrcBmp {
                        img,
                        palette,
                        dimension,
                    },
                )| {
                    // with_file_name would overwrite the extension
                    // regardless, we will overwrite the extension at the end
                    let path = if self.options.is_tiling {
                        let current_file_name =
                            path.file_stem().unwrap().to_str().unwrap().to_string();
                        path.with_file_name(format!(
                            "{}_{}",
                            current_file_name, self.options.tiling_scalar
                        ))
                    } else {
                        path.to_path_buf()
                    };

                    let path = if self.options.is_transparent {
                        let current_file_name =
                            path.file_stem().unwrap().to_str().unwrap().to_string();
                        path.with_file_name(format!("{{{}", current_file_name))
                    } else {
                        path
                    };

                    let path = path.with_extension("bmp");

                    if let Err(err) = write_8bpp_to_file(&img, &palette, dimension, &path) {
                        let err_str = format!("Error writing file {}: {}", path.display(), err);

                        return Err(eyre!(err_str));
                    }

                    Ok(())
                },
            )
            .collect::<Vec<eyre::Result<()>>>();

        if let Some(Err(err)) = write_res.iter().find(|res| res.is_err()) {
            let err_str = err.to_string();

            self.log(&err_str);

            return Err(eyre!(err_str));
        }

        if let Some(sync) = &self.sync {
            *sync.status.lock().unwrap() = "Done".to_string();
        }

        Ok(())
    }
}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn run() {
        let textile = TexTileBuilder::new(vec![
            "/home/khang/gchimp/examples/textile/gridwall_glow.png".into(),
        ])
        .tiling(true)
        .tiling_scalar(2)
        .transparent(false)
        .transparent_threshold(0.75)
        .change_name(true)
        .work();

        println!("{:?}", textile);

        assert!(textile.is_ok())
    }
}
