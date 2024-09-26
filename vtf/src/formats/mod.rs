use dxt::{Dxt1, Dxt5};
use image::DynamicImage;
use nom::{combinator::fail, error::context};
use rgb8::{bgr888::Bgr888, rgb888::Rgb888};

use crate::{IResult, ImageData};

pub mod dxt;
pub mod rgb8;

mod utils;

pub trait VtfImageImpl {
    fn parse(i: &[u8], dimensions: (u32, u32)) -> IResult<ImageData>;
    fn to_image(bytes: &[u8], dimensions: (u32, u32)) -> DynamicImage;
}

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum VtfImageFormat {
    None = -1,
    Rgba8888 = 0,
    Abgr8888,
    Rgb888,
    Bgr888,
    Rgb565,
    I8,
    Ia88,
    P8,
    A8,
    Rgb888Bluescreen,
    Bgr888Bluescreen,
    Argb8888,
    Bgra8888,
    Dxt1,
    Dxt3,
    Dxt5,
    Bgrx8888,
    Bgr565,
    Bgrx5551,
    Bgra4444,
    Dxt1Onebitalpha,
    Bgra5551,
    Uv88,
    Uvwq8888,
    Rgba16161616f,
    Rgba16161616,
    Uvlx8888,
}

#[derive(Debug, Clone)]
pub struct VtfImage {
    pub format: VtfImageFormat,
    pub dimensions: (u32, u32),
    pub bytes: Vec<u8>,
}

impl TryFrom<i32> for VtfImageFormat {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(VtfImageFormat::None),
            0 => Ok(VtfImageFormat::Rgba8888),
            1 => Ok(VtfImageFormat::Abgr8888),
            2 => Ok(VtfImageFormat::Rgb888),
            3 => Ok(VtfImageFormat::Bgr888),
            4 => Ok(VtfImageFormat::Rgb565),
            5 => Ok(VtfImageFormat::I8),
            6 => Ok(VtfImageFormat::Ia88),
            7 => Ok(VtfImageFormat::P8),
            8 => Ok(VtfImageFormat::A8),
            9 => Ok(VtfImageFormat::Rgb888Bluescreen),
            10 => Ok(VtfImageFormat::Bgr888Bluescreen),
            11 => Ok(VtfImageFormat::Argb8888),
            12 => Ok(VtfImageFormat::Bgra8888),
            13 => Ok(VtfImageFormat::Dxt1),
            14 => Ok(VtfImageFormat::Dxt3),
            15 => Ok(VtfImageFormat::Dxt5),
            16 => Ok(VtfImageFormat::Bgrx8888),
            17 => Ok(VtfImageFormat::Bgr565),
            18 => Ok(VtfImageFormat::Bgrx5551),
            19 => Ok(VtfImageFormat::Bgra4444),
            20 => Ok(VtfImageFormat::Dxt1Onebitalpha),
            21 => Ok(VtfImageFormat::Bgra5551),
            22 => Ok(VtfImageFormat::Uv88),
            23 => Ok(VtfImageFormat::Uvwq8888),
            24 => Ok(VtfImageFormat::Rgba16161616f),
            25 => Ok(VtfImageFormat::Rgba16161616),
            26 => Ok(VtfImageFormat::Uvlx8888),
            _ => Err("not a valid image format"),
        }
    }
}

impl VtfImage {
    pub fn parse_from_format(
        i: &[u8],
        format: VtfImageFormat,
        dimensions: (u32, u32),
    ) -> IResult<VtfImage> {
        let mut not_supported = context("vtf image format not supported", fail);

        let (i, bytes) = match format {
            VtfImageFormat::None => not_supported(i),
            VtfImageFormat::Rgba8888 => not_supported(i),
            VtfImageFormat::Abgr8888 => not_supported(i),
            VtfImageFormat::Rgb888 => Rgb888::parse(i, dimensions),
            VtfImageFormat::Bgr888 => Bgr888::parse(i, dimensions),
            VtfImageFormat::Rgb565 => not_supported(i),
            VtfImageFormat::I8 => not_supported(i),
            VtfImageFormat::Ia88 => not_supported(i),
            VtfImageFormat::P8 => not_supported(i),
            VtfImageFormat::A8 => not_supported(i),
            VtfImageFormat::Rgb888Bluescreen => not_supported(i),
            VtfImageFormat::Bgr888Bluescreen => not_supported(i),
            VtfImageFormat::Argb8888 => not_supported(i),
            VtfImageFormat::Bgra8888 => not_supported(i),
            VtfImageFormat::Dxt1 => Dxt1::parse(i, dimensions),
            VtfImageFormat::Dxt3 => not_supported(i),
            VtfImageFormat::Dxt5 => Dxt5::parse(i, dimensions),
            VtfImageFormat::Bgrx8888 => not_supported(i),
            VtfImageFormat::Bgr565 => not_supported(i),
            VtfImageFormat::Bgrx5551 => not_supported(i),
            VtfImageFormat::Bgra4444 => not_supported(i),
            VtfImageFormat::Dxt1Onebitalpha => not_supported(i),
            VtfImageFormat::Bgra5551 => not_supported(i),
            VtfImageFormat::Uv88 => not_supported(i),
            VtfImageFormat::Uvwq8888 => not_supported(i),
            VtfImageFormat::Rgba16161616f => not_supported(i),
            VtfImageFormat::Rgba16161616 => not_supported(i),
            VtfImageFormat::Uvlx8888 => not_supported(i),
        }
        .unwrap();

        Ok((
            i,
            Self {
                format,
                dimensions,
                bytes,
            },
        ))
    }

    pub fn to_image(&self) -> DynamicImage {
        match self.format {
            VtfImageFormat::None => todo!(),
            VtfImageFormat::Rgba8888 => todo!(),
            VtfImageFormat::Abgr8888 => todo!(),
            VtfImageFormat::Rgb888 => Rgb888::to_image(&self.bytes, self.dimensions),
            VtfImageFormat::Bgr888 => Bgr888::to_image(&self.bytes, self.dimensions),
            VtfImageFormat::Rgb565 => todo!(),
            VtfImageFormat::I8 => todo!(),
            VtfImageFormat::Ia88 => todo!(),
            VtfImageFormat::P8 => todo!(),
            VtfImageFormat::A8 => todo!(),
            VtfImageFormat::Rgb888Bluescreen => todo!(),
            VtfImageFormat::Bgr888Bluescreen => todo!(),
            VtfImageFormat::Argb8888 => todo!(),
            VtfImageFormat::Bgra8888 => todo!(),
            VtfImageFormat::Dxt1 => Dxt1::to_image(&self.bytes, self.dimensions),
            VtfImageFormat::Dxt3 => todo!(),
            VtfImageFormat::Dxt5 => Dxt5::to_image(&self.bytes, self.dimensions),
            VtfImageFormat::Bgrx8888 => todo!(),
            VtfImageFormat::Bgr565 => todo!(),
            VtfImageFormat::Bgrx5551 => todo!(),
            VtfImageFormat::Bgra4444 => todo!(),
            VtfImageFormat::Dxt1Onebitalpha => todo!(),
            VtfImageFormat::Bgra5551 => todo!(),
            VtfImageFormat::Uv88 => todo!(),
            VtfImageFormat::Uvwq8888 => todo!(),
            VtfImageFormat::Rgba16161616f => todo!(),
            VtfImageFormat::Rgba16161616 => todo!(),
            VtfImageFormat::Uvlx8888 => todo!(),
        }
    }
}
