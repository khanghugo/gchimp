use nom::bytes::complete::take;

use crate::types::{IResult, ImageData};

pub mod i8;

fn parse_u8(i: &'_ [u8], dimensions: (u32, u32)) -> IResult<'_, ImageData> {
    let (width, height) = dimensions;

    let bit_count = (width) * height * 8; // 8 bpp
    let byte_count = bit_count.div_ceil(8);

    // // max 1 bytes
    // // because if the dimensions is (0, 1) because of mipmap
    // // still minimum 1 bytes taken
    // // otherwise, it would take 0 bytes beacuse 0 * 1 = 0
    // let byte_count = byte_count.max(1);

    let (i, bytes) = take(byte_count)(i)?;

    Ok((i, bytes.to_vec()))
}
