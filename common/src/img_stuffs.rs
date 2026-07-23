use std::{
    collections::{HashMap, HashSet},
    f32,
    fs::OpenOptions,
    io::{BufReader, BufWriter, Cursor, Write},
    path::{Path, PathBuf},
};

use eyre::eyre;
use image::{
    GenericImageView, ImageDecoder, RgbImage, Rgba32FImage, RgbaImage, codecs::bmp::BmpDecoder,
    imageops,
};
use quantette::{ColorSpace, ImagePipeline, QuantizeMethod};
use rayon::prelude::*;
use vtf::Vtf;

use crate::constants::MAX_GOLDSRC_TEXTURE_SIZE;

type Palette = Vec<quantette::palette::rgb::Rgb<quantette::palette::encoding::Srgb, u8>>;

const OTHER_OBVIOUS_TRANSPARENT_COLORS: &[[u8; 3]] = &[
    [0, 255, 0],   // green
    [0, 0, 255],   // blue
    [255, 0, 255], // magenta
];

/// The pixels are quantized with following palette.
///
/// ## Must convert image to 8bpp with the palette.
fn quantize_image(img: RgbImage) -> eyre::Result<(RgbImage, Palette)> {
    let mut binding = ImagePipeline::try_from(&img)?;

    let pipeline = binding
        .palette_size(255)
        .dither(true)
        .colorspace(ColorSpace::Srgb)
        .quantize_method(QuantizeMethod::kmeans());

    let img = pipeline.clone().quantized_rgbimage_par();
    let palette: Palette = pipeline.palette_par();

    Ok((img, palette))
}

pub fn maybe_resize_due_to_exceeding_max_goldsrc_texture_size(img: &RgbaImage) -> RgbaImage {
    let (width, height) = img.dimensions();

    let bigger_side = if width >= height { width } else { height };
    let q = bigger_side as f32 / MAX_GOLDSRC_TEXTURE_SIZE as f32;

    let make_multiple_of_16 = |(width, height): (u32, u32)| {
        let (need_width, need_height) = ((16 - width % 16) % 16, (16 - height % 16) % 16);

        (
            (width + need_width).min(MAX_GOLDSRC_TEXTURE_SIZE),
            (height + need_height).min(MAX_GOLDSRC_TEXTURE_SIZE),
        )
    };

    // must use nearest filter to avoid making new colors
    if q <= 1. {
        // make sure that is is multiple of 16
        let (new_width, new_height) = make_multiple_of_16((width, height));

        // good enough? i guess?
        imageops::resize(img, new_width, new_height, imageops::FilterType::Nearest)
    } else {
        let (width, height) = (width as f32 / q, height as f32 / q);
        let (width, height) = (width.round() as u32, height.round() as u32);
        let (width, height) = make_multiple_of_16((width, height));

        imageops::resize(
            img,
            width,
            height,
            // eh, meow?
            imageops::FilterType::Nearest,
        )
    }
}

fn get_palette_from_rgbaimage(img: &RgbaImage) -> Vec<[u8; 3]> {
    let mut hash_set = HashSet::<[u8; 3]>::new();

    img.pixels().for_each(|pixel| {
        hash_set.insert([pixel.0[0], pixel.0[1], pixel.0[2]]);
    });

    hash_set.into_iter().collect()
}

// if alpha is 0, then replace it with transparent color
// otherwise, linearly blend the pixel
fn rgba8_to_rgb8(img: RgbaImage) -> eyre::Result<RgbImage> {
    let (width, height) = img.dimensions();

    let palette = get_palette_from_rgbaimage(&img);
    let transparent_color = get_a_color_that_does_not_exist(&palette, 10);

    let buf = img
        .par_chunks_exact(4)
        .flat_map(|p| {
            let should_replace = p[3] == 0;

            if should_replace {
                [
                    transparent_color[0],
                    transparent_color[1],
                    transparent_color[2],
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
    let img = generate_rgba8_from_image_path(img_path.as_ref())?;
    let rgba8 = maybe_resize_due_to_exceeding_max_goldsrc_texture_size(&img);
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
    let img = generate_rgba8_from_image_path(img_path.as_ref())?;
    let rgba8 = maybe_resize_due_to_exceeding_max_goldsrc_texture_size(&img);
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
pub fn encode_8bpp_to_bitmap_bytes(
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

    // resize here can use higher quality lanczos3 because
    // we care more about perceptivity than
    imageops::resize(&res, width, height, imageops::FilterType::Lanczos3)
}

fn get_a_color_that_does_not_exist(palette: &[[u8; 3]], count: usize) -> [u8; 3] {
    // count to help with getting multiple colors
    let mut local_count = 0;

    // not 255 inclusive so it does not exceed 255 when adding
    for base in (0..255u8).rev() {
        // starting from very bright color because it is easier to dither
        // the problem right now is that if the color is too dark, this color will be part of the dithering
        // resulting in transparent pixel being dark dots
        for r in [0, 255] {
            for g in [0, 255] {
                for b in [0, 255] {
                    let curr_color = [
                        if r == 255 { base } else { 0 },
                        if g == 255 { base } else { 0 },
                        if b == 255 { base } else { 0 },
                    ];

                    if !palette.contains(&curr_color) {
                        if local_count == count {
                            return curr_color;
                        }

                        local_count += 1;
                    }
                }
            }
        }
    }

    // it is not possible for this to happen
    [0, 0, 255] // but still use blue
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
    let pad_color = get_a_color_that_does_not_exist(palette, 0);
    new_palette.resize(256, pad_color);

    // if we can't use these obvious transparent colors, then just fall back to adaptive color stuff
    let maybe_have_obvious_transparent_colors = OTHER_OBVIOUS_TRANSPARENT_COLORS
        .iter()
        .find(|current_trans| new_palette.iter().all(|x| x != *current_trans))
        .cloned();

    // change the final color of the palette to a color that is not in the palette
    let transparent_color = maybe_have_obvious_transparent_colors
        .unwrap_or(get_a_color_that_does_not_exist(palette, 1));
    new_palette[255] = transparent_color;

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
    let mip0 = maybe_resize_due_to_exceeding_max_goldsrc_texture_size(&img);
    let quantize_res = rgba8_to_8bpp(mip0);

    if let Err(err) = quantize_res {
        return Err(eyre!("Cannot quantize image: {}", err));
    }

    let GoldSrcBmp {
        image: mip0,
        palette,
        dimensions: (width, height),
    } = quantize_res.unwrap();

    // must use nearest filter type here
    // to avoid making new color palette
    let (mip1, _, _) = generate_indexed_mipmap(&mip0, width, height, 1);
    let (mip2, _, _) = generate_indexed_mipmap(&mip0, width, height, 2);
    let (mip3, _, _) = generate_indexed_mipmap(&mip0, width, height, 3);

    Ok(GenerateMipmapsResult {
        mips: [mip0, mip1, mip2, mip3],
        palette,
        dimensions: (width, height),
    })
}

// thanks gemini
fn generate_indexed_mipmap(
    src_pixels: &[u8],
    src_width: u32,
    src_height: u32,
    level: u32,
) -> (Vec<u8>, u32, u32) {
    let step = 1 << level; // 2^level stride
    let target_width = (src_width >> level).max(1);
    let target_height = (src_height >> level).max(1);

    let mut pixels = Vec::with_capacity(target_width as usize * target_height as usize);

    for y in 0..(target_height as usize) {
        let row_offset = (y * step) * src_width as usize;

        for x in 0..(target_width as usize) {
            pixels.push(src_pixels[row_offset + (x * step)]);
        }
    }

    (pixels, target_width, target_height)
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

    let img = generate_rgba8_from_image_path(img_path.as_ref())?;

    generate_mipmaps_from_rgba_image(img)
}

pub fn generate_rgba8_from_image_path(
    img_path: impl AsRef<Path> + Into<PathBuf>,
) -> eyre::Result<image::RgbaImage> {
    let ext = img_path.as_ref().extension().unwrap();

    let img = if ext == "vtf" {
        Vtf::from_file(img_path.as_ref())?
            .get_high_res_image()?
            .into_rgba8()
    } else {
        image::open(img_path.as_ref())?.into_rgba8()
    };

    Ok(img)
}

#[derive(Debug)]
pub struct GoldSrcBmp {
    pub image: Vec<u8>,
    pub palette: Vec<[u8; 3]>,
    pub dimensions: (u32, u32),
}

impl GoldSrcBmp {
    pub fn pad_palette(&mut self) {
        self.palette.resize(256, [0; 3]);
    }
}

// written by both gemini and deepseek with great debug work from your truly
pub fn hdri_to_cubemap(
    hdri: &Rgba32FImage,
    cube_dimension: u32,
    exposure: f32,
) -> [(&'static str, RgbaImage); 6] {
    let mut faces = [
        ("up", RgbaImage::new(cube_dimension, cube_dimension)), // 0: U
        ("lf", RgbaImage::new(cube_dimension, cube_dimension)), // 1: L
        ("bk", RgbaImage::new(cube_dimension, cube_dimension)), // 2: B
        ("rt", RgbaImage::new(cube_dimension, cube_dimension)), // 3: R
        ("ft", RgbaImage::new(cube_dimension, cube_dimension)), // 4: F
        ("dn", RgbaImage::new(cube_dimension, cube_dimension)), // 5: D
    ];

    let hdri_w = hdri.width() as f32;
    let hdri_h = hdri.height() as f32;

    for (face_idx, face) in faces.iter_mut().enumerate() {
        for y in 0..cube_dimension {
            for x in 0..cube_dimension {
                // Convert pixel to [-1, 1] range
                let a = (2.0 * (x as f32 + 0.5)) / cube_dimension as f32 - 1.0;
                let b = (2.0 * (y as f32 + 0.5)) / cube_dimension as f32 - 1.0;

                // Z-up cube face directions
                // Using standard Z-up convention: X=right, Y=forward, Z=up
                let [px, py, pz] = match face_idx {
                    0 => [b, a, 1.0],    // up
                    1 => [-1.0, -a, -b], // lf
                    2 => [-a, 1.0, -b],  // bk
                    3 => [1.0, a, -b],   // rt
                    4 => [a, -1.0, -b],  // ft
                    5 => [-b, a, -1.0],  // dn
                    _ => unreachable!(),
                };

                // Normalize
                let len = (px * px + py * py + pz * pz).sqrt();
                let (nx, ny, nz) = (px / len, py / len, pz / len);

                // Convert to spherical coordinates (Z-up)
                // theta: angle in XY plane from Y axis, phi: angle from Z axis
                let theta = f32::atan2(nx, ny); // [-PI, PI]
                let phi = f32::acos(nz); // [0, PI]

                // Map to HDRI UV: u = longitude, v = latitude
                let u = (theta + f32::consts::PI) / (2.0 * f32::consts::PI);
                let v = phi / f32::consts::PI; // 0 at top (Z+), 1 at bottom (Z-)

                // Sample HDRI
                let color = sample_bilinear(hdri, u * hdri_w, v * hdri_h, hdri_w, hdri_h);

                // Apply exposure and tone mapping
                let color_exp0 = color[0] * exposure;
                let color_exp1 = color[1] * exposure;
                let color_exp2 = color[2] * exposure;

                let color_toned0 = color_exp0 / (1.0 + color_exp0);
                let color_toned1 = color_exp1 / (1.0 + color_exp1);
                let color_toned2 = color_exp2 / (1.0 + color_exp2);

                let color0 = (color_toned0 * 255.0).clamp(0.0, 255.0) as u8;
                let color1 = (color_toned1 * 255.0).clamp(0.0, 255.0) as u8;
                let color2 = (color_toned2 * 255.0).clamp(0.0, 255.0) as u8;

                face.1.put_pixel(x, y, [color0, color1, color2, 255].into());
            }
        }
    }

    faces
}

/// Helper function to perform bilinear interpolation on the HDRI map
fn sample_bilinear(hdri: &Rgba32FImage, x: f32, y: f32, width: f32, height: f32) -> [f32; 4] {
    // Determine the coordinates of the 4 surrounding pixels
    let x0 = (x.floor() as i32).rem_euclid(width as i32) as u32;
    let y0 = (y.floor() as i32).clamp(0, height as i32 - 1) as u32;
    let x1 = ((x0 + 1) as f32).rem_euclid(width) as u32;
    let y1 = ((y0 + 1) as f32).clamp(0.0, height - 1.0) as u32;

    // Interpolation weights
    let fx = x.fract();
    let fy = y.fract();

    // Get the 4 pixels
    let p00 = hdri.get_pixel(x0, y0).0;
    let p10 = hdri.get_pixel(x1, y0).0;
    let p00_1 = hdri.get_pixel(x0, y1).0; // p01
    let p11 = hdri.get_pixel(x1, y1).0;

    // Interpolate channels
    let mut rgbaf32 = [0.; 4];
    for i in 0..4 {
        let c00 = p00[i];
        let c10 = p10[i];
        let c01 = p00_1[i];
        let c11 = p11[i];

        // Bilinear blend formula
        let top = c00 + fx * (c10 - c00);
        let bottom = c01 + fx * (c11 - c01);
        let final_val = top + fy * (bottom - top);

        rgbaf32[i] = final_val;
    }

    rgbaf32
}

pub fn adjust_hdri_exposure(hdri: &RgbaImage, exposure: f32) -> RgbaImage {
    let mut adjusted = hdri.clone();

    // Calculate the multiplier: 2^exposure
    // Exposure of 1.0 doubles the brightness, -1.0 halves it, 0.0 does nothing.
    let multiplier = 2.0f32.powf(exposure);

    for pixel in adjusted.pixels_mut() {
        // Leave the Alpha channel (index 3) untouched
        for i in 0..3 {
            let original_val = pixel.0[i] as f32;

            // Apply exposure and clamp to valid u8 bounds [0, 255]
            let new_val = (original_val * multiplier).clamp(0.0, 255.0);

            pixel.0[i] = new_val as u8;
        }
    }

    adjusted
}
