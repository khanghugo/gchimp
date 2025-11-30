use nom::bytes::complete::take;

use super::*;

pub mod bgra8888;

fn parse_rgba8888(i: &'_ [u8], dimensions: (u32, u32)) -> IResult<'_, ImageData> {
    let (width, height) = dimensions;

    let bit_count = (width) * height * 32; // 32 bpp
    let byte_count = bit_count.div_ceil(8);

    // max 4 bytes
    // because if the dimensions is (0, 1) because of mipmap
    // still minimum 4 bytes taken
    // otherwise, it would take 0 bytes beacuse 0 * 1 = 0
    let byte_count = byte_count.max(4);

    let (i, bytes) = take(byte_count)(i)?;

    Ok((i, bytes.to_vec()))
}
