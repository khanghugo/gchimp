use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{BufReader, BufWriter, Cursor, Write},
    path::{Path, PathBuf},
};

use eyre::eyre;
use image::{
    codecs::bmp::BmpDecoder, imageops, GenericImageView, ImageDecoder, RgbImage, RgbaImage,
};
use quantette::{ColorSpace, ImagePipeline, QuantizeMethod};
use rayon::prelude::*;
use vtf::Vtf;

use crate::utils::constants::MAX_GOLDSRC_TEXTURE_SIZE;

use super::constants::{PALETTE_PAD_COLOR, PALETTE_TRANSPARENT_COLOR, PALETTE_TRANSPARENT_COLOR2};

use crate::err;

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

    let make_multiple_of_16 = |(width, height): (u32, u32)| {
        let (need_width, need_height) = (16 - width % 16, 16 - height % 16);

        // if image is already multiple of 16, don't go up
        let need_width = if need_width == 16 { 0 } else { need_width };
        let need_height = if need_height == 16 { 0 } else { need_height };

        (
            (width + need_width).min(MAX_GOLDSRC_TEXTURE_SIZE),
            (height + need_height).min(MAX_GOLDSRC_TEXTURE_SIZE),
        )
    };

    if q <= 1. {
        // make sure that is is multiple of 16
        let (new_width, new_height) = make_multiple_of_16((width, height));

        // good enough? i guess?
        imageops::resize(&img, new_width, new_height, imageops::FilterType::Lanczos3)
    } else {
        let (width, height) = (width as f32 / q, height as f32 / q);
        let (width, height) = (width.round() as u32, height.round() as u32);
        let (width, height) = make_multiple_of_16((width, height));

        imageops::resize(
            &img,
            width,
            height,
            // eh, meow?
            imageops::FilterType::Lanczos3,
        )
    }
}

// if alpha is 0, then replace it with transparent color
// otherwise, linearly blend the pixel
fn rgba8_to_rgb8(img: RgbaImage) -> eyre::Result<RgbImage> {
    let (width, height) = img.dimensions();
    let buf = img
        .par_chunks_exact(4)
        .flat_map(|p| {
            let should_replace = p[3] <= 64;

            if should_replace {
                [
                    PALETTE_TRANSPARENT_COLOR2[0],
                    PALETTE_TRANSPARENT_COLOR2[1],
                    PALETTE_TRANSPARENT_COLOR2[2],
                ]
            } else {
                let opacity = p[3] as f32 / 255.;
                [
                    (p[0] as f32 * opacity).round() as u8,
                    (p[1] as f32 * opacity).round() as u8,
                    (p[2] as f32 * opacity).round() as u8,
                ]
            }
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

pub fn any_format_to_bmp_write_to_file(
    img_path: impl AsRef<Path> + Into<PathBuf>,
) -> eyre::Result<()> {
    let img = image::open(img_path.as_ref())?.into_rgba8();
    let rgba8 = maybe_resize_due_to_exceeding_max_goldsrc_texture_size(img);
    let GoldSrcBmp {
        image: img,
        palette,
        dimensions: dimension,
    } = rgba8_to_8bpp(rgba8)?;

    let mut out_img = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(img_path.as_ref().with_extension("bmp"))?;

    let mut encoder = image::codecs::bmp::BmpEncoder::new(&mut out_img);

    encoder.encode_with_palette(
        &img,
        dimension.0,
        dimension.1,
        image::ExtendedColorType::L8,
        Some(&palette),
    )?;

    out_img.flush()?;

    Ok(())
}

pub fn any_format_to_png(
    img_path: impl AsRef<Path> + Into<PathBuf>,
) -> eyre::Result<(Vec<u8>, (u32, u32))> {
    let img = image::open(img_path.as_ref())?;
    let dimensions = img.dimensions();

    let mut buf: Vec<u8> = vec![];

    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)?;

    Ok((buf, dimensions))
}

pub fn any_format_to_8bpp(img_path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<GoldSrcBmp> {
    let img = image::open(img_path.as_ref())?.into_rgba8();
    let rgba8 = maybe_resize_due_to_exceeding_max_goldsrc_texture_size(img);
    let res = rgba8_to_8bpp(rgba8)?;

    Ok(res)
}

pub fn png_to_bmp_folder(paths: &[PathBuf]) -> eyre::Result<()> {
    let err: Vec<eyre::Error> = paths
        .par_iter()
        .filter_map(|path| any_format_to_bmp_write_to_file(path).err())
        .collect();

    if !err.is_empty() {
        let err_str = err
            .iter()
            .fold(String::new(), |acc, e| format!("{}\n{}", acc, e));

        return Err(eyre::eyre!(err_str));
    }

    Ok(())
}

pub fn rgba8_to_8bpp(rgb8a: RgbaImage) -> eyre::Result<GoldSrcBmp> {
    // TODO convert totally opaque pixel into transparent pixel
    let rgb8 = rgba8_to_rgb8(rgb8a)?;
    let (rgb8, palette_color) = quantize_image(rgb8)?;

    let dimension = rgb8.dimensions();

    let palette_color_arr = format_quantette_palette(palette_color);
    let img_bmp_8pp = rgb8_to_8bpp(rgb8, &palette_color_arr);

    Ok(GoldSrcBmp {
        image: img_bmp_8pp,
        palette: palette_color_arr,
        dimensions: dimension,
    })
}

/// `file_name` should have .bmp have extension
pub fn write_8bpp_to_file(
    img: &[u8],
    palette: &[[u8; 3]],
    dimension: (u32, u32),
    file_path: impl AsRef<Path>,
) -> eyre::Result<()> {
    assert!(file_path.as_ref().extension().is_some());
    assert!(file_path.as_ref().extension().unwrap() == "bmp");

    let mut out_img = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(file_path)?;

    let mut writer = BufWriter::new(&mut out_img);
    let mut encoder = image::codecs::bmp::BmpEncoder::new(&mut writer);

    encoder.encode_with_palette(
        img,
        dimension.0,
        dimension.1,
        image::ExtendedColorType::L8,
        Some(palette),
    )?;

    writer.flush()?;

    Ok(())
}

// need to encode it into a format so it has all the info to easily display every where
pub fn encode_8bpp_to_bitmap(
    img: &[u8],
    palette: &[[u8; 3]],
    dimension: (u32, u32),
) -> eyre::Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut encoder = image::codecs::bmp::BmpEncoder::new(&mut buf);

    encoder.encode_with_palette(
        img,
        dimension.0,
        dimension.1,
        image::ExtendedColorType::L8,
        Some(palette),
    )?;

    Ok(buf)
}

// egui doesn't take bitmap, what in tarnation?
pub fn eight_bpp_bitmap_to_png_bytes(
    img: &[u8],
    palette: &[[u8; 3]],
    (width, height): (u32, u32),
) -> eyre::Result<Vec<u8>> {
    let img_buffer = img
        .iter()
        .flat_map(|palette_index| palette[*palette_index as usize])
        .collect::<Vec<u8>>();
    let img_buffer = RgbImage::from_vec(width, height, img_buffer).unwrap();

    let mut bytes: Vec<u8> = Vec::new();
    img_buffer.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)?;

    Ok(bytes)
}

// tile the image in a way that the resulting image has the same dimension as the original
pub fn tile_and_resize(img: &RgbaImage, scalar: u32) -> RgbaImage {
    let (width, height) = img.dimensions();
    let mut res = RgbaImage::new(width * scalar, height * scalar);

    imageops::tile(&mut res, img);

    // this doesn't modify the original image, LOL wtf
    imageops::resize(&res, width, height, imageops::FilterType::Lanczos3)
}

// threshold is between 0 and 1
// if the percentage of the most used color is over the threshold, mark the color transparent
pub fn eight_bpp_transparent_img(
    img: &[u8],
    palette: &[[u8; 3]],
    threshold: f32,
) -> (Vec<u8>, Vec<[u8; 3]>) {
    // find most used color and its count
    let (most_used_color, most_used_color_count) = img
        .iter()
        .fold(HashMap::<u8, usize>::new(), |mut acc, p| {
            if let Some(count) = acc.get_mut(p) {
                *count += 1;
            } else {
                acc.insert(*p, 1);
            }

            acc
        })
        .iter()
        .fold((0, 0), |(acc_pixel, acc_count), (pixel, count)| {
            if *count > acc_count {
                (*pixel, *count)
            } else {
                (acc_pixel, acc_count)
            }
        });

    let over_threshold = most_used_color_count as f32 / img.len() as f32 >= threshold;

    if !over_threshold {
        return (img.to_vec(), palette.to_vec());
    }

    let mut new_palette = palette.to_vec();
    let mut new_img = img.to_vec();

    // pad palette
    new_palette.resize(256, PALETTE_PAD_COLOR);

    // change the final color of the palette to a rare color
    new_palette[255] = PALETTE_TRANSPARENT_COLOR;

    // swap the most used color (index) with 255
    for pixel in new_img.iter_mut() {
        if *pixel == most_used_color {
            *pixel = 255;
        }
    }

    (new_img, new_palette)
}

pub struct GenerateMipmapsResult {
    pub mips: [Vec<u8>; 4],
    pub palette: Vec<[u8; 3]>,
    pub dimensions: (u32, u32),
}

// the idea is that if the input file is a bitmap, just don't do anything :)
#[allow(clippy::type_complexity)]
fn _generate_mipmaps_indexed_bmp(
    img_path: impl AsRef<Path> + Into<PathBuf>,
) -> eyre::Result<GenerateMipmapsResult> {
    let img_file = OpenOptions::new().read(true).open(img_path.as_ref())?;

    let decoder = BmpDecoder::new(BufReader::new(img_file))?;

    let (width, height) = decoder.dimensions();
    let palette = decoder.get_palette();

    if palette.is_none() {
        return Err(eyre!("This is not an indexed bitmap."));
    }

    let palette = palette.unwrap().to_owned();

    let buf_len = width * height * 3;
    let mut mip0 = vec![0u8; buf_len as usize];

    decoder.read_image(&mut mip0)?;

    let mip0 = mip0
        .chunks(3)
        .map(|pixel| {
            palette
                .iter()
                .position(|curr_pal| curr_pal == &[pixel[0], pixel[1], pixel[2]])
                .unwrap() as u8
        })
        .collect::<Vec<u8>>();
    let mip1 = vec![0u8; width as usize * height as usize / 4];
    let mip2 = vec![0u8; width as usize * height as usize / 4 / 4];
    let mip3 = vec![0u8; width as usize * height as usize / 4 / 4 / 4];

    Ok(GenerateMipmapsResult {
        mips: [mip0, mip1, mip2, mip3],
        palette,
        dimensions: (width, height),
    })
}

// TODO: better mipmaps generation because this is very SHIT
pub fn generate_mipmaps_from_rgba_image(img: RgbaImage) -> eyre::Result<GenerateMipmapsResult> {
    let mip0 = maybe_resize_due_to_exceeding_max_goldsrc_texture_size(img);

    let mip0 = rgba8_to_rgb8(mip0);

    if let Err(err) = mip0 {
        return err!("Cannot convert rgba8 to rgb8: {}", err);
    }

    let mip0 = mip0.unwrap();

    let quantize_res = quantize_image(mip0);

    if let Err(err) = quantize_res {
        return err!("Cannot quantize image: {}", err);
    }

    let (mip0, palette_color) = quantize_res.unwrap();

    let (width, height) = mip0.dimensions();

    let palette = format_quantette_palette(palette_color);

    let mip1 = imageops::resize(&mip0, width / 2, height / 2, imageops::FilterType::Nearest);
    let mip2 = imageops::resize(
        &mip0,
        width / 2 / 2,
        height / 2 / 2,
        imageops::FilterType::Nearest,
    );
    let mip3 = imageops::resize(
        &mip0,
        width / 2 / 2 / 2,
        height / 2 / 2 / 2,
        imageops::FilterType::Nearest,
    );

    let mip0 = rgb8_to_8bpp(mip0, &palette);
    let mip1 = rgb8_to_8bpp(mip1, &palette);
    let mip2 = rgb8_to_8bpp(mip2, &palette);
    let mip3 = rgb8_to_8bpp(mip3, &palette);

    Ok(GenerateMipmapsResult {
        mips: [mip0, mip1, mip2, mip3],
        palette,
        dimensions: (width, height),
    })
}

pub fn generate_mipmaps_from_path(
    img_path: impl AsRef<Path> + Into<PathBuf>,
) -> eyre::Result<GenerateMipmapsResult> {
    let ext = img_path.as_ref().extension().unwrap();

    // if it is bitmap, then for now don't generate any bitmap
    // dont even do any thing really.
    // just return the original image and mipmaps are dummy
    // unless the bitmap is not indexed, then process normally
    if ext == "bmp" {
        let res = _generate_mipmaps_indexed_bmp(img_path.as_ref());

        if let Ok(res) = res {
            return Ok(res);
        }
    };

    let img = if ext == "vtf" {
        Vtf::from_file(img_path.as_ref())?
            .get_high_res_image()?
            .into_rgba8()
    } else {
        image::open(img_path.as_ref())?.into_rgba8()
    };

    generate_mipmaps_from_rgba_image(img)
}

#[derive(Debug)]
pub struct GoldSrcBmp {
    pub image: Vec<u8>,
    pub palette: Vec<[u8; 3]>,
    pub dimensions: (u32, u32),
}
