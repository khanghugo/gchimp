use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use eyre::eyre;
use image::{imageops, RgbImage, RgbaImage};
use quantette::{ColorSpace, ImagePipeline, QuantizeMethod};
use rayon::prelude::*;

use super::constants::MAX_GAME_TEXTURE_SIZE;

type Palette = Vec<quantette::palette::rgb::Rgb<quantette::palette::encoding::Srgb, u8>>;

fn quantize_to_8pp(img: RgbImage) -> eyre::Result<(RgbImage, Palette)> {
    let pipeline = ImagePipeline::try_from(&img)?
        .palette_size(255)
        .dither(true)
        .colorspace(ColorSpace::Oklab)
        .quantize_method(QuantizeMethod::kmeans());

    let img = pipeline.clone().quantized_rgbimage_par();
    let palette: Palette = pipeline.palette_par();

    Ok((img, palette))
}

fn maybe_resize(img: RgbImage) -> RgbImage {
    let (width, height) = img.dimensions();

    let bigger_side = if width >= height { width } else { height };
    let q = bigger_side as f32 / MAX_GAME_TEXTURE_SIZE as f32;

    if q <= 1. {
        img
    } else {
        let (width, height) = (width as f32 / q, height as f32 / q);
        let (width, height) = (width.round() as u32, height.round() as u32);
        imageops::resize(
            &img,
            width,
            height,
            // eh, meow?
            imageops::FilterType::Nearest,
        )
    }
}

fn rgba_to_rgb(img: RgbaImage) -> eyre::Result<RgbImage> {
    let (width, height) = img.dimensions();
    let buf = img
        .par_chunks_exact(4)
        .flat_map(|p| {
            let opacity = p[3] as f32 / 255.;
            [
                (p[0] as f32 * opacity).round() as u8,
                (p[1] as f32 * opacity).round() as u8,
                (p[2] as f32 * opacity).round() as u8,
            ]
        })
        .collect::<Vec<u8>>();

    let res = match RgbImage::from_vec(width, height, buf) {
        Some(buf) => Ok(buf),
        None => Err(eyre!("Cannot convert Rgba to Rgb")),
    }?;

    Ok(res)
}

fn png_to_bmp(img_path: &Path) -> eyre::Result<()> {
    let img = image::open(img_path)?.into_rgba8();
    let img = rgba_to_rgb(img)?;
    let img = maybe_resize(img);
    let (width, height) = img.dimensions();
    let (img, palette_color) = quantize_to_8pp(img)?;

    let mut out_img = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(img_path.with_extension("bmp"))?;

    let mut encoder = image::codecs::bmp::BmpEncoder::new(&mut out_img);

    let palette_color_arr = palette_color
        .iter()
        .map(|p| [p.red, p.green, p.blue])
        .collect::<Vec<[u8; 3]>>();

    // converting 24bpp into 8bpp with the palette we have
    let img_bmp_8pp = img
        .chunks_exact(3)
        .map(|p| {
            // unwrap is guaranteed because img uses palette colors
            let index_for_color = palette_color_arr
                .iter()
                .position(|pp| *pp == [p[0], p[1], p[2]])
                .unwrap();
            index_for_color as u8
        })
        .collect::<Vec<u8>>();

    encoder.encode_with_palette(
        &img_bmp_8pp,
        width,
        height,
        image::ExtendedColorType::L8,
        Some(&palette_color_arr),
    )?;

    out_img.flush()?;

    Ok(())
}

pub fn png_to_bmp_par(paths: &[PathBuf]) -> eyre::Result<()> {
    let err: Vec<eyre::Error> = paths
        .par_iter()
        .filter_map(|path| png_to_bmp(path).err())
        .collect();

    if !err.is_empty() {
        let err_str = err
            .iter()
            .fold(String::new(), |acc, e| format!("{}\n{}", acc, e));

        return Err(eyre::eyre!(err_str));
    }

    Ok(())
}
