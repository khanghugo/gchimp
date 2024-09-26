use nom::bytes::complete::take;

use super::*;

pub mod bgr888;
pub mod rgb888;

fn parse_rgb888(i: &[u8], dimensions: (u32, u32)) -> IResult<ImageData> {
    let (width, height) = dimensions;

    let bit_count = (width) * height * 24; // 24 bpp
    let byte_count = bit_count.div_ceil(8);

    // max 3 bytes
    // because if the dimensions is (0, 1) because of mipmap
    // still minimum 3 bytes taken
    // otherwise, it would take 0 bytes beacuse 0 * 1 = 0
    let byte_count = byte_count.max(3);

    let (i, bytes) = take(byte_count)(i)?;

    Ok((i, bytes.to_vec()))
}
