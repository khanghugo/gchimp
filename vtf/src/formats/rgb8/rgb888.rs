use utils::rgb8_buffer_to_image;

use super::*;

pub struct Rgb888;

impl VtfImageImpl for Rgb888 {
    fn parse(i: &'_ [u8], dimensions: (u32, u32)) -> IResult<'_, ImageData> {
        parse_rgb888(i, dimensions)
    }

    fn to_image(bytes: &[u8], dimensions: (u32, u32)) -> DynamicImage {
        let (width, height) = dimensions;

        let buf = bytes
            .chunks(3)
            .map(|rgb| [rgb[0], rgb[1], rgb[2]])
            .collect::<Vec<[u8; 3]>>();

        rgb8_buffer_to_image(&buf, width, height)
    }
}
