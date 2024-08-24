use image::{ImageBuffer, Rgb, RgbImage};
use nom::bytes::complete::take;
use utils::{pack_rgb888, rgb8_buffer_to_image, unpack_rgb565};

use super::*;

pub struct Dxt1;

impl VtfImageImpl for Dxt1 {
    fn parse(i: &[u8], dimensions: (u32, u32)) -> IResult<ImageData> {
        let (width, height) = dimensions;

        let bit_count = width * height * 4; // 4 bpp
        let byte_count = bit_count / 8;

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
            .map(|block| {
                let c0 = u16::from_le_bytes([block[0], block[1]]);
                let c1 = u16::from_le_bytes([block[2], block[3]]);

                let (cp0, cp1) = (unpack_rgb565(c0), unpack_rgb565(c1));

                let (cp2, cp3) = if c0 > c1 {
                    (
                        [
                            cp0[0] * 2 / 3 + cp1[0] / 3,
                            cp0[1] * 2 / 3 + cp1[1] / 3,
                            cp0[2] * 2 / 3 + cp1[2] / 3,
                        ],
                        [
                            cp0[0] / 3 + cp1[0] * 2 / 3,
                            cp0[1] / 3 + cp1[1] * 2 / 3,
                            cp0[2] / 3 + cp1[2] * 2 / 3,
                        ],
                    )
                } else {
                    (
                        [
                            cp0[0] / 2 + cp1[0] / 2,
                            cp0[1] / 2 + cp1[1] / 2,
                            cp0[2] / 2 + cp1[2] / 2,
                        ],
                        [0, 0, 0],
                    )
                };

                let look_up = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);

                // 4x4
                (0..16)
                    .map(|idx| {
                        let shift_value = 30 - idx * 2;
                        let lookup_value = (look_up << shift_value) >> 30;

                        if lookup_value == 0 {
                            pack_rgb888(cp0)
                        } else if lookup_value == 1 {
                            pack_rgb888(cp1)
                        } else if lookup_value == 2 {
                            pack_rgb888(cp2)
                        } else if lookup_value == 3 {
                            pack_rgb888(cp3)
                        } else {
                            unreachable!("not a valid look up value")
                        }
                    })
                    .collect::<Vec<[u8; 3]>>()
            })
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
