use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    str::from_utf8,
};

use rayon::prelude::*;

use eyre::eyre;
use wad::{Entry, Wad};

use crate::utils::img_stuffs::{
    eight_bpp_bitmap_to_png_bytes, generate_mipmaps, write_8bpp_to_file,
};

pub struct Waddy {
    wad: Wad,
}

impl Default for Waddy {
    fn default() -> Self {
        Self::new()
    }
}

impl Waddy {
    pub fn new() -> Self {
        Self { wad: Wad::new() }
    }

    pub fn from_file(path: impl AsRef<Path> + Into<PathBuf> + AsRef<OsStr>) -> eyre::Result<Self> {
        let wad = Wad::from_file(path)?;

        Ok(Waddy { wad })
    }

    pub fn from_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        let wad = Wad::from_bytes(bytes)?;

        Ok(Waddy { wad })
    }

    fn log(&self, i: impl std::fmt::Display + AsRef<str>) {
        println!("{}", i);
    }

    pub fn wad(&self) -> &Wad {
        &self.wad
    }

    /// Returns the info of the WAD file including header and non-content
    pub fn dump_info(&self) -> String {
        let mut res = String::new();

        // basic header
        res += format!(
            "Version: {}\n",
            from_utf8(self.wad.header.magic.as_slice()).unwrap()
        )
        .as_str();

        res += format!("Number of textures: {}\n\n", self.wad.header.num_dirs).as_str();

        // image data
        self.wad
            .entries
            .iter()
            .enumerate()
            .for_each(|(index, entry)| {
                let (width, height) = entry.file_entry.dimensions();

                res += format!(
                    "{index:<4}: {:<16} {:>3}x{:<3}\n",
                    entry.texture_name(),
                    width,
                    height
                )
                .as_str();
            });
        res
    }

    // egui cannot parse 8bpp bitmap
    pub fn dump_textures_to_png_bytes(&self) -> eyre::Result<Vec<(usize, Vec<u8>)>> {
        let res = self
            .wad
            .entries
            .par_iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let (width, height) = entry.file_entry.dimensions();

                let res = eight_bpp_bitmap_to_png_bytes(
                    entry.file_entry.image(),
                    entry.file_entry.palette(),
                    (width, height),
                );

                if let Ok(img) = res {
                    return Some((index, img));
                }

                None
            })
            .collect::<Vec<(usize, Vec<u8>)>>();

        if res.len() != self.wad.header.num_dirs as usize {
            let err_str = format!(
                "Cannot parse all of textures ({}/{})",
                res.len(),
                self.wad.header.num_dirs
            );

            self.log(&err_str);

            return Err(eyre!(err_str));
        }

        Ok(res)
    }

    pub fn dump_texture_to_file(
        &self,
        texture_index: usize,
        out_path_file: impl AsRef<Path> + Into<PathBuf> + Sync,
    ) -> eyre::Result<()> {
        if out_path_file.as_ref().parent().is_none()
            || !out_path_file.as_ref().parent().unwrap().exists()
        {
            return Err(eyre!("Output folder does not exist"));
        }

        let res = self
            .wad
            .entries
            .get(texture_index)
            .map(|entry| match &entry.file_entry {
                wad::FileEntry::Qpic(_) => unimplemented!(),
                wad::FileEntry::MipTex(miptex) => {
                    let res = write_8bpp_to_file(
                        miptex.mip_images[0].data.get_bytes(),
                        miptex.palette.get_bytes(),
                        (miptex.width, miptex.height),
                        out_path_file.as_ref().with_extension("bmp"),
                    );

                    if let Err(err) = res {
                        let err_str = format!(
                            "Error writing {}: {}",
                            out_path_file.as_ref().display(),
                            err
                        );
                        return Some(err_str);
                    }

                    None
                }
                wad::FileEntry::Font(_) => unimplemented!(),
            });

        if res.is_none() {
            let err_str = format!(
                "Index {} out of bound {}",
                texture_index,
                self.wad.header.num_dirs - 1
            );

            self.log(&err_str);

            return Err(eyre!(err_str));
        } else if let Some(err_str) = res.unwrap() {
            self.log(&err_str);

            return Err(eyre!(err_str));
        }

        Ok(())
    }

    /// Dumps all textures into .bmp format to a specified folder
    pub fn dump_textures_to_files(
        &self,
        path: impl AsRef<Path> + Into<PathBuf> + Sync,
    ) -> eyre::Result<()> {
        if !path.as_ref().exists() {
            return Err(eyre!("Output folder does not exist"));
        }

        let res = self
            .wad
            .entries
            .par_iter()
            .filter_map(|entry| match &entry.file_entry {
                wad::FileEntry::Qpic(_) => unimplemented!(),
                wad::FileEntry::MipTex(miptex) => {
                    let out_path = path
                        .as_ref()
                        .join(miptex.texture_name.get_string())
                        .with_extension("bmp");
                    let res = write_8bpp_to_file(
                        miptex.mip_images[0].data.get_bytes(),
                        miptex.palette.get_bytes(),
                        (miptex.width, miptex.height),
                        &out_path,
                    );

                    if let Err(err) = res {
                        let err_str = format!("Error writing {}: {}", out_path.display(), err);
                        return Some(err_str);
                    }

                    None
                }
                wad::FileEntry::Font(_) => unimplemented!(),
            })
            .collect::<Vec<String>>();

        if !res.is_empty() {
            let err_str = res
                .iter()
                .fold(String::new(), |acc, e| format!("{acc}\n{e}\n"));

            self.log(&err_str);

            return Err(eyre!(err_str));
        }

        Ok(())
    }

    pub fn rename_texture(
        &mut self,
        texture_index: usize,
        s: impl AsRef<str> + Into<String> + Clone,
    ) -> eyre::Result<()> {
        self.wad.entries[texture_index].set_name(s)
    }

    pub fn remove_texture(&mut self, texture_index: usize) {
        self.wad.header.num_dirs = (self.wad.header.num_dirs - 1).max(0);
        self.wad.entries.remove(texture_index);
    }

    pub fn add_texture(&mut self, path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<()> {
        let res = generate_mipmaps(path.as_ref());

        if let Err(err) = res {
            let err_str = format!(
                "Cannot convert {} to 8bpp: {}",
                path.as_ref().display(),
                err
            );

            self.log(&err_str);

            return Err(eyre!(err_str));
        }

        let texture_name = path.as_ref().file_stem().unwrap().to_str().unwrap();

        self.wad.header.num_dirs += 1;

        let ([mip0, mip1, mip2, mip3], palette, dimensions) = res.unwrap();

        let new_entry = Entry::new(
            texture_name,
            dimensions,
            &[&mip0, &mip1, &mip2, &mip3],
            palette.as_slice(),
        );

        self.wad.entries.push(new_entry);

        Ok(())
    }

    pub fn save_to_file(&self, path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<()> {
        self.wad.write_to_file(path)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn dump_info() {
        let waddy = Waddy::from_file("/home/khang/gchimp/wad/test/surf_cyberwave.wad").unwrap();
        println!("{}", waddy.dump_info());
    }

    #[test]
    fn dump_info2() {
        let waddy = Waddy::from_file("/home/khang/map_compiler/cso_normal_pack.wad").unwrap();
        println!("{}", waddy.dump_info());
    }

    #[test]
    fn dump_textures() {
        let waddy = Waddy::from_file("/home/khang/gchimp/wad/test/surf_cyberwave.wad").unwrap();
        waddy
            .dump_textures_to_files("/home/khang/gchimp/examples/waddy/")
            .unwrap();
    }

    #[test]
    fn dump_textures2() {
        {
            let waddy = Waddy::from_file("/home/khang/map_compiler/cso_normal_pack.wad").unwrap();

            waddy
                .dump_textures_to_files("/home/khang/gchimp/examples/waddy/cso")
                .unwrap();
        }

        // check the memory usage
        std::thread::sleep(std::time::Duration::from_secs(15));
    }

    #[test]
    fn add_wad() {
        let mut waddy = Waddy::from_file("/home/khang/gchimp/examples/waddy/wad_test.wad").unwrap();

        // waddy
        //     .add_texture("/home/khang/map_compiler/my_textures/black.bmp")
        //     .unwrap();

        // waddy
        //     .add_texture("/home/khang/gchimp/examples/waddy/cyberwave/neon_blueing..bmp")
        //     .unwrap();

        waddy
            .add_texture("/home/khang/gchimp/examples/waddy/cyberwave/z.bmp")
            .unwrap();

        waddy
            .save_to_file("/home/khang/gchimp/examples/waddy/wad_test_out.wad")
            .unwrap();
    }
}
