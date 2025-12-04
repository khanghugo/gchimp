use std::{
    ffi::OsStr,
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    str::from_utf8,
};

use bsp::Bsp;
use image::RgbaImage;
use rayon::prelude::*;

use eyre::eyre;
use wad::types::{Entry, FileEntry, Wad};

use crate::utils::img_stuffs::{
    eight_bpp_bitmap_to_png_bytes, generate_mipmaps_from_path, generate_mipmaps_from_rgba_image,
    write_8bpp_to_file, GenerateMipmapsResult,
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

    pub fn from_wad_file(
        path: impl AsRef<Path> + Into<PathBuf> + AsRef<OsStr>,
    ) -> eyre::Result<Self> {
        let wad = Wad::from_file(path)?;

        Ok(Waddy { wad })
    }

    pub fn from_wad_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        let wad = Wad::from_bytes(bytes)?;

        Ok(Waddy { wad })
    }

    pub fn from_bsp_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        let mut res = Self::new();

        let bsp = Bsp::from_bytes(bytes)?;
        let textures = bsp.textures;

        // TODO maybe one day I will change this at wad write level
        res.wad.header.num_dirs = textures.len() as i32;

        res.wad.entries = textures
            .into_iter()
            .map(|texture| {
                let texture_name = texture.texture_name.get_string();

                wad::types::Entry {
                    directory_entry: wad::types::DirectoryEntry::new(texture_name),
                    file_entry: wad::types::FileEntry::MipTex(texture),
                }
            })
            .collect::<Vec<wad::types::Entry>>();

        Ok(res)
    }

    pub fn from_bsp_file(
        path: impl AsRef<Path> + Into<PathBuf> + AsRef<OsStr>,
    ) -> eyre::Result<Self> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let mut bytes = vec![];

        file.read_to_end(&mut bytes)?;

        Self::from_bsp_bytes(&bytes)
    }

    fn log(&self, i: impl std::fmt::Display + AsRef<str>) {
        println!("{}", i);
    }

    pub fn wad(&self) -> &Wad {
        &self.wad
    }

    pub fn wad_mut(&mut self) -> &mut Wad {
        &mut self.wad
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
                FileEntry::Qpic(_) => unimplemented!(),
                FileEntry::MipTex(miptex) => {
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
                FileEntry::Font(_) => unimplemented!(),
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
            .filter_map(|entry| {
                let out_file_name = entry.texture_name();
                let out_path = path.as_ref().join(out_file_name).with_extension("bmp");

                match &entry.file_entry {
                    FileEntry::Qpic(qpic) => {
                        let res = write_8bpp_to_file(
                            qpic.data.get_bytes(),
                            qpic.palette.get_bytes(),
                            qpic.dimensions(),
                            &out_path,
                        );

                        if let Err(err) = res {
                            let err_str = format!("Error writing {}: {}", out_path.display(), err);
                            return Some(err_str);
                        }

                        None
                    }
                    FileEntry::MipTex(miptex) => {
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
                    FileEntry::Font(font) => {
                        let res = write_8bpp_to_file(
                            font.data.get_bytes(),
                            font.palette.get_bytes(),
                            font.dimensions(),
                            &out_path,
                        );

                        if let Err(err) = res {
                            let err_str = format!("Error writing {}: {}", out_path.display(), err);
                            return Some(err_str);
                        }

                        None
                    }
                }
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

    fn add_texture_from_generated_mipmaps(
        &mut self,
        texture_name: &str,
        res: GenerateMipmapsResult,
    ) {
        let GenerateMipmapsResult {
            mips: [mip0, mip1, mip2, mip3],
            palette,
            dimensions,
        } = res;

        let new_entry = Entry::new(
            texture_name,
            dimensions,
            &[&mip0, &mip1, &mip2, &mip3],
            palette.as_slice(),
        );

        // remember to add numb_dirs explicitly....
        // TODO maybe don't do this and have the writer write the numdirs for us
        self.wad.header.num_dirs += 1;

        self.wad.entries.push(new_entry);
    }

    pub fn add_texture_from_rgba_image(
        &mut self,
        texture_name: &str,
        image: RgbaImage,
    ) -> eyre::Result<()> {
        let res = generate_mipmaps_from_rgba_image(image)?;

        self.add_texture_from_generated_mipmaps(texture_name, res);

        Ok(())
    }

    pub fn add_texture_from_image_path(
        &mut self,
        path: impl AsRef<Path> + Into<PathBuf>,
    ) -> eyre::Result<()> {
        let res = generate_mipmaps_from_path(path.as_ref())?;

        let texture_name = path.as_ref().file_stem().unwrap().to_str().unwrap();

        self.add_texture_from_generated_mipmaps(texture_name, res);

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
    fn add_wad() {
        let mut waddy = Waddy::new();

        let img1 = include_bytes!("../../test/neon_red.bmp");
        let img2 = include_bytes!("../../test/neon_yellow.bmp");
        let img3 = include_bytes!("../../test/rainbow.vtf");

        let img3_vtf = vtf::Vtf::from_bytes(img3).unwrap();

        let img1_dynamic = image::load_from_memory(img1).unwrap();
        let img2_dynamic = image::load_from_memory(img2).unwrap();
        let img3_dynamic = img3_vtf.get_high_res_image().unwrap();

        waddy
            .add_texture_from_rgba_image("neon_red", img1_dynamic.into())
            .unwrap();
        waddy
            .add_texture_from_rgba_image("neon_yellow", img2_dynamic.into())
            .unwrap();
        waddy
            .add_texture_from_rgba_image("rainbow", img3_dynamic.into())
            .unwrap();

        assert_eq!(waddy.wad.entries.len(), 3);

        {
            let entry = &waddy.wad.entries[0];
            assert_eq!(entry.texture_name_standard(), "NEON_RED");
            assert_eq!(entry.file_entry.dimensions(), (16, 16));
        }

        {
            let entry = &waddy.wad.entries[1];
            assert_eq!(entry.texture_name_standard(), "NEON_YELLOW");
            assert_eq!(entry.file_entry.dimensions(), (16, 16));
        }

        {
            let entry = &waddy.wad.entries[2];
            assert_eq!(entry.texture_name_standard(), "RAINBOW");
            assert_eq!(entry.file_entry.dimensions(), (512, 512));
        }
    }

    #[test]
    fn open_bsp() {
        let bsp_bytes = include_bytes!("../../test/datacore.bsp");
        let waddy = Waddy::from_bsp_bytes(bsp_bytes).unwrap();

        assert_eq!(waddy.wad.entries.len(), 94);

        assert!(waddy.wad.entries.iter().all(|entry| entry.is_external()))
    }

    #[test]
    fn open_bsp2() {
        let bsp_bytes = include_bytes!("../../test/kz_pinkblaus.bsp");
        let waddy = Waddy::from_bsp_bytes(bsp_bytes).unwrap();

        assert_eq!(waddy.wad.entries.len(), 83);
        assert!(waddy.wad.entries.iter().all(|entry| !entry.is_external()))
    }

    #[ignore]
    #[test]
    fn dump_tx() {
        let bytes = include_bytes!("/home/khang/bxt/_game_native/valve/gfx.wad");

        let waddy = Waddy::from_wad_bytes(bytes).unwrap();

        waddy.dump_textures_to_files("/tmp/aaaa/").unwrap();
    }
}
