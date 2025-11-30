use crate::formats::utils::rgb8_buffer_to_image;

use super::*;

pub struct Uv88;

impl VtfImageImpl for Uv88 {
    fn parse(i: &'_ [u8], dimensions: (u32, u32)) -> IResult<'_, ImageData> {
        parse_rg88(i, dimensions)
    }

    fn to_image(bytes: &[u8], dimensions: (u32, u32)) -> DynamicImage {
        let (width, height) = dimensions;

        let buf = bytes
            .chunks(2)
            .map(|uv| [uv[0], uv[1], 0])
            .collect::<Vec<[u8; 3]>>();

        rgb8_buffer_to_image(&buf, width, height)
    }
}
