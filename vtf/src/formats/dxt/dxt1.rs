use nom::bytes::complete::take;
use utils::rgb8_buffer_to_image;

use super::*;

pub struct Dxt1;

impl VtfImageImpl for Dxt1 {
    fn parse(i: &'_ [u8], dimensions: (u32, u32)) -> IResult<'_, ImageData> {
        let (width, height) = dimensions;

        let bit_count = width * height * 4; // 4 bpp
        let byte_count = bit_count.div_ceil(8);

        // smaller mipmap sizes such as 1x1 or 2x2 still take full 8 bytes
        let byte_count = byte_count.max(8);

        let (i, bytes) = take(byte_count)(i)?;

        Ok((i, bytes.to_vec()))
    }

    fn to_image(bytes: &[u8], dimensions: (u32, u32)) -> DynamicImage {
        let (width, height) = dimensions;
        let column_count = (width as usize / 4).max(1);

        let pixels = bytes
            .chunks(8) // 32 bit for color, 32 bit for 4x4 2 bit look up
            .map(dxt_color_block_to_color_pixels)
            // vector of vector of pixel here means vector of 4x4 blocks
            .collect::<Vec<Vec<[u8; 3]>>>()
            // each pixel chunk is 4x4
            // so now each chunk here is a row of pixel
            .chunks(column_count)
            .flat_map(|row| {
                // 4 rows per 4x4 chunk
                (0..4).flat_map(|pixel_row_idx| {
                    row.iter()
                        .flat_map(|pixel_chunk| {
                            pixel_chunk[(pixel_row_idx * 4)..((pixel_row_idx + 1) * 4)].to_vec()
                        })
                        .collect::<Vec<[u8; 3]>>()
                })
            })
            .collect::<Vec<[u8; 3]>>();

        rgb8_buffer_to_image(&pixels, width, height)
    }
}
