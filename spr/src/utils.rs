use std::{ffi::OsStr, fs::OpenOptions, io::Write, path::Path};

use image::RgbImage;
use nom::Parser;

use crate::{error::SprError, parser::parse_spr, Spr};

impl Spr {
    pub fn open_from_bytes(i: &[u8]) -> Result<Spr, SprError> {
        parse_spr
            .parse(i)
            .map_err(move |op| SprError::NomError {
                source: op.to_owned(),
            })
            .map(move |(_, res)| res)
    }

    pub fn open_from_file(path: impl AsRef<OsStr> + AsRef<Path>) -> Result<Spr, SprError> {
        let file = std::fs::read(path).map_err(|op| SprError::IOError { source: op })?;

        Self::open_from_bytes(&file)
    }

    pub fn write_to_file(&self, path: impl AsRef<OsStr> + AsRef<Path>) -> Result<(), SprError> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)
            .map_err(|op| SprError::IOError { source: op })?;

        file.write_all(self.write_to_bytes().as_slice())
            .map_err(|op| SprError::IOError { source: op })?;

        Ok(())
    }

    pub fn to_rgb8(&self, frame_index: usize) -> RgbImage {
        let frame = &self.frames[frame_index];
        let stride_length = frame.header.width as u32;
        let mut image = RgbImage::new(frame.header.width as u32, frame.header.height as u32);

        image.enumerate_rows_mut().for_each(|(_, pixels_row)| {
            pixels_row.for_each(|(width, height, pixel)| {
                let color_index = frame.image[(width + height * stride_length) as usize];
                let color = self.palette[color_index as usize];
                *pixel = color.into();
            })
        });

        image
    }
}
