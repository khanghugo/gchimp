use dxt::{Dxt1, Dxt5};
use image::DynamicImage;
use nom::{
    combinator::{cut, fail},
    error::context,
    Parser,
};
use rgb8::{bgr888::Bgr888, rgb888::Rgb888};

use crate::{formats::u8::U8, IResult, ImageData};

pub mod dxt;
pub mod rgb8;
pub mod u8;

mod utils;

pub trait VtfImageImpl {
    fn parse(i: &'_ [u8], dimensions: (u32, u32)) -> IResult<'_, ImageData>;
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
        i: &'_ [u8],
        format: VtfImageFormat,
        dimensions: (u32, u32),
    ) -> IResult<'_, VtfImage> {
        let (i, bytes) = match format {
            VtfImageFormat::Rgb888 => Rgb888::parse(i, dimensions),
            VtfImageFormat::Bgr888 => Bgr888::parse(i, dimensions),
            VtfImageFormat::Dxt1 => Dxt1::parse(i, dimensions),
            VtfImageFormat::Dxt5 => Dxt5::parse(i, dimensions),
            VtfImageFormat::I8 => U8::parse(i, dimensions),
            VtfImageFormat::A8 => U8::parse(i, dimensions),
            VtfImageFormat::P8 => U8::parse(i, dimensions),
            not_supported => {
                println!("image format not supported {:?}", not_supported);
                return context("vtf image format not supported", cut(fail)).parse(&[]);
            }
        }?;

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
            VtfImageFormat::Rgb888 => Rgb888::to_image(&self.bytes, self.dimensions),
            VtfImageFormat::Bgr888 => Bgr888::to_image(&self.bytes, self.dimensions),
            VtfImageFormat::Dxt1 => Dxt1::to_image(&self.bytes, self.dimensions),
            VtfImageFormat::Dxt5 => Dxt5::to_image(&self.bytes, self.dimensions),
            VtfImageFormat::I8 => U8::to_image(&self.bytes, self.dimensions),
            VtfImageFormat::A8 => U8::to_image(&self.bytes, self.dimensions),
            VtfImageFormat::P8 => U8::to_image(&self.bytes, self.dimensions),
            _ => todo!(),
        }
    }
}
