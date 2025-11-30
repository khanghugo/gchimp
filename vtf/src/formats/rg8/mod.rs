use nom::bytes::complete::take;

use super::*;

pub mod uv88;

fn parse_rg88(i: &'_ [u8], dimensions: (u32, u32)) -> IResult<'_, ImageData> {
    let (width, height) = dimensions;

    let bit_count = (width) * height * 16; // 16 bpp
    let byte_count = bit_count.div_ceil(8);

    // max 2 bytes
    // because if the dimensions is (0, 1) because of mipmap
    // still minimum 2 bytes taken
    // otherwise, it would take 0 bytes beacuse 0 * 1 = 0
    let byte_count = byte_count.max(2);

    let (i, bytes) = take(byte_count)(i)?;

    Ok((i, bytes.to_vec()))
}
