use crate::formats::{u8::parse_u8, utils::rgb8_buffer_to_image, VtfImageImpl};

pub struct I8;
impl VtfImageImpl for I8 {
    fn parse(
        i: &'_ [u8],
        dimensions: (u32, u32),
    ) -> crate::types::IResult<'_, crate::types::ImageData> {
        parse_u8(i, dimensions)
    }

    fn to_image(bytes: &[u8], dimensions: (u32, u32)) -> image::DynamicImage {
        let (width, height) = dimensions;

        let buf = bytes.iter().map(|&v| [v; 3]).collect::<Vec<[u8; 3]>>();

        rgb8_buffer_to_image(&buf, width, height)
    }
}
