use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use eyre::eyre;
use wad::{FileEntry, Wad};

use super::img_stuffs::write_8bpp_to_file;

#[derive(Clone, Default)]
pub struct SimpleWadEntry {
    // instead of storing the name, for now just store the index instead
    // this implies we have to keep track of the orginal Vec<Wad> to index correctly
    wad_file_index: usize,
    dimensions: (u32, u32),
}

impl SimpleWadEntry {
    pub fn wad_file_index(&self) -> usize {
        self.wad_file_index
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }
}

#[derive(Clone, Default)]
/// Just WAD(s) data indexed by texture name
pub struct SimpleWad(HashMap<String, SimpleWadEntry>);

impl From<&[&Wad]> for SimpleWad {
    fn from(value: &[&Wad]) -> Self {
        let mut res = Self::default();

        value.iter().enumerate().for_each(|(wad_file_index, wad)| {
            wad.entries.iter().for_each(|entry| {
                if let FileEntry::MipTex(miptex) = &entry.file_entry {
                    res.0.insert(
                        entry.directory_entry.texture_name.get_string(),
                        SimpleWadEntry {
                            wad_file_index,
                            dimensions: (miptex.width, miptex.height),
                        },
                    );
                }
            });
        });

        res
    }
}

impl From<&[Wad]> for SimpleWad {
    fn from(value: &[Wad]) -> Self {
        value.iter().collect::<Vec<&Wad>>().into()
    }
}

impl From<Vec<Wad>> for SimpleWad {
    fn from(value: Vec<Wad>) -> Self {
        value.as_slice().into()
    }
}

impl From<Vec<&Wad>> for SimpleWad {
    fn from(value: Vec<&Wad>) -> Self {
        value.as_slice().into()
    }
}

impl SimpleWad {
    pub fn from_wads(value: &[Wad]) -> Self {
        value.into()
    }

    pub fn get(&self, k: &str) -> Option<&SimpleWadEntry> {
        self.0.get(k)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &SimpleWadEntry)> {
        self.0.iter()
    }

    pub fn insert(&mut self, k: impl AsRef<str> + Into<String>, index: usize, v: (u32, u32)) {
        self.0.insert(
            k.into(),
            SimpleWadEntry {
                wad_file_index: index,
                dimensions: v,
            },
        );
    }
}

/// Exports a WAD texture from given name to an indexed bitmap
pub fn export_texture(
    wad: &Wad,
    texture_name: &str,
    out_path_file: impl AsRef<Path> + Into<PathBuf> + Sync,
) -> eyre::Result<()> {
    let res = wad
        .entries
        .iter()
        .find(|entry| entry.texture_name() == texture_name)
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
        return Err(eyre!("Cannot find texture: {}", texture_name));
    } else if let Some(err_str) = res.unwrap() {
        return Err(eyre!("{}", err_str));
    }

    Ok(())
}
