use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use eyre::eyre;
use image::{imageops, RgbImage, RgbaImage};
use quantette::{ColorSpace, ImagePipeline, QuantizeMethod};
use rayon::prelude::*;

use crate::utils::constants::MAX_GOLDSRC_TEXTURE_SIZE;

type Palette = Vec<quantette::palette::rgb::Rgb<quantette::palette::encoding::Srgb, u8>>;

/// The pixels are quantized with following palette.
///
/// ## Must convert image to 8bpp with the palette.
fn quantize_image(img: RgbImage) -> eyre::Result<(RgbImage, Palette)> {
    let pipeline = ImagePipeline::try_from(&img)?
        .palette_size(255)
        .dither(true)
        .colorspace(ColorSpace::Oklab)
        .quantize_method(QuantizeMethod::kmeans());

    let img = pipeline.clone().quantized_rgbimage_par();
    let palette: Palette = pipeline.palette_par();

    Ok((img, palette))
}

fn maybe_resize_due_to_exceeding_max_goldsrc_texture_size(img: RgbaImage) -> RgbaImage {
    let (width, height) = img.dimensions();

    let bigger_side = if width >= height { width } else { height };
    let q = bigger_side as f32 / MAX_GOLDSRC_TEXTURE_SIZE as f32;

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
            imageops::FilterType::Lanczos3,
        )
    }
}

fn rgba8_to_rgb8(img: RgbaImage) -> eyre::Result<RgbImage> {
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

fn format_quantette_palette(palette: Palette) -> Vec<[u8; 3]> {
    palette
        .iter()
        .map(|p| [p.red, p.green, p.blue])
        .collect::<Vec<[u8; 3]>>()
}

fn rgb8_to_8bpp(img: RgbImage, palette: &[[u8; 3]]) -> Vec<u8> {
    img.chunks_exact(3)
        .map(|p| {
            // unwrap is guaranteed because img uses palette colors
            let index_for_color = palette
                .iter()
                .position(|pp| *pp == [p[0], p[1], p[2]])
                .unwrap();
            index_for_color as u8
        })
        .collect::<Vec<u8>>()
}

pub fn png_to_bmp(img_path: &Path) -> eyre::Result<()> {
    let img = image::open(img_path)?.into_rgba8();
    let rgba8 = maybe_resize_due_to_exceeding_max_goldsrc_texture_size(img);
    let (width, height) = rgba8.dimensions();
    let (eight_bpp, eight_bpp_palette) = rgba8_to_8bpp(rgba8)?;

    let mut out_img = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(img_path.with_extension("bmp"))?;

    let mut encoder = image::codecs::bmp::BmpEncoder::new(&mut out_img);

    encoder.encode_with_palette(
        &eight_bpp,
        width,
        height,
        image::ExtendedColorType::L8,
        Some(&eight_bpp_palette),
    )?;

    out_img.flush()?;

    Ok(())
}

pub fn png_to_bmp_folder(paths: &[PathBuf]) -> eyre::Result<()> {
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

pub fn rgba8_to_8bpp(rgb8a: RgbaImage) -> eyre::Result<(Vec<u8>, Vec<[u8; 3]>)> {
    let rgb8 = rgba8_to_rgb8(rgb8a)?;
    let (rgb8, palette_color) = quantize_image(rgb8)?;

    let palette_color_arr = format_quantette_palette(palette_color);
    let img_bmp_8pp = rgb8_to_8bpp(rgb8, &palette_color_arr);

    Ok((img_bmp_8pp, palette_color_arr))
}

/// `file_name` should not have extension
pub fn write_8bpp(
    img: &[u8],
    palette: &[[u8; 3]],
    dimension: (u32, u32),
    file_name: &str,
) -> eyre::Result<()> {
    let path = PathBuf::from(file_name);

    let mut out_img = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path.with_extension("bmp"))?;

    let mut encoder = image::codecs::bmp::BmpEncoder::new(&mut out_img);

    encoder.encode_with_palette(
        img,
        dimension.0,
        dimension.1,
        image::ExtendedColorType::L8,
        Some(palette),
    )?;

    out_img.flush()?;

    Ok(())
}
