use std::{
    ffi::OsStr,
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

use bsp::Bsp;
use wad::types::{Entry, Wad};

pub fn bsp2wad_bytes(bsp_bytes: &[u8]) -> eyre::Result<Vec<u8>> {
    let bsp = Bsp::from_bytes(bsp_bytes)?;
    let textures = bsp.textures;

    let mut out_wad = Wad::new();

    textures.iter().for_each(|texture| {
        let mip_maps = texture
            .mip_images
            .iter()
            .map(|image| image.data.get_bytes().as_slice())
            .collect::<Vec<&[u8]>>();

        let new_entry = Entry::new(
            texture.texture_name.get_string_standard(),
            (texture.width, texture.height),
            &mip_maps,
            texture.palette.get_bytes().as_slice(),
        );

        out_wad.header.num_dirs += 1;
        out_wad.entries.push(new_entry);
    });

    Ok(out_wad.write_to_bytes())
}

pub fn bsp2wad(path: impl AsRef<OsStr> + AsRef<Path>) -> eyre::Result<()> {
    let bsp_path: &Path = path.as_ref();

    let mut bsp_file = OpenOptions::new().read(true).open(bsp_path)?;
    let mut bsp_bytes: Vec<u8> = vec![];

    bsp_file.read_to_end(&mut bsp_bytes)?;

    let res_bytes = bsp2wad_bytes(&bsp_bytes)?;

    let mut out_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(bsp_path.with_extension("wad"))?;

    out_file.write_all(&res_bytes)?;
    out_file.flush()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::bsp2wad;

    #[test]
    fn run() {
        bsp2wad("/home/khang/bxt/game_isolated/valve/maps/c1a0.bsp").unwrap();
    }
}
