use crate::formats::utils::rgba8_buffer_to_image;

use super::*;

pub struct Bgra8888;

impl VtfImageImpl for Bgra8888 {
    fn parse(i: &'_ [u8], dimensions: (u32, u32)) -> IResult<'_, ImageData> {
        parse_rgba8888(i, dimensions)
    }

    fn to_image(bytes: &[u8], dimensions: (u32, u32)) -> DynamicImage {
        let (width, height) = dimensions;

        let buf = bytes
            .chunks(4)
            .map(|bgra| [bgra[2], bgra[1], bgra[0], bgra[3]])
            .collect::<Vec<[u8; 4]>>();

        rgba8_buffer_to_image(&buf, width, height)
    }
}
