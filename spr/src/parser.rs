use nom::{
    bytes::complete::take,
    combinator::map,
    multi::count,
    number::complete::{le_f32, le_i16, le_i32, le_u8},
    IResult as _IResult, Parser,
};

use crate::{Spr, SprFrame, SprFrameHeader, SprFrameImage, SprFrames, SprHeader, SprPalette};

pub type IResult<'a, T> = _IResult<&'a [u8], T>;

pub fn parse_header(i: &'_ [u8]) -> IResult<'_, SprHeader> {
    map(
        (
            le_i32, le_i32, le_i32, le_i32, le_f32, le_i32, le_i32, le_i32, le_f32, le_i32, le_i16,
        ),
        |(
            id,
            version,
            orientation,
            texture_format,
            bounding_radius,
            max_width,
            max_height,
            frame_num,
            beam_length,
            sync_type,
            palette_count,
        )| SprHeader {
            id,
            version,
            orientation,
            texture_format,
            bounding_radius,
            max_width,
            max_height,
            frame_num,
            beam_length,
            sync_type,
            palette_count,
        },
    )
    .parse(i)
}

pub fn parse_palette(i: &'_ [u8], palette_count: usize) -> IResult<'_, SprPalette> {
    count(
        map(take(3usize), |arr: &[u8]| [arr[0], arr[1], arr[2]]),
        palette_count,
    )
    .parse(i)
}

pub fn parse_frame_header(i: &'_ [u8]) -> IResult<'_, SprFrameHeader> {
    map(
        (le_i32, le_i32, le_i32, le_i32, le_i32),
        |(group, origin_x, origin_y, width, height)| SprFrameHeader {
            group,
            origin_x,
            origin_y,
            width,
            height,
        },
    )
    .parse(i)
}

pub fn parse_frame_image(i: &'_ [u8], length: usize) -> IResult<'_, SprFrameImage> {
    count(le_u8, length).parse(i)
}

pub fn parse_frame(i: &'_ [u8]) -> IResult<'_, SprFrame> {
    let (i, header) = parse_frame_header.parse(i)?;
    let image_length = (header.width * header.height) as usize;
    let (i, image) = parse_frame_image(i, image_length)?;

    Ok((i, SprFrame { header, image }))
}

pub fn parse_frames(i: &'_ [u8], frame_count: usize) -> IResult<'_, SprFrames> {
    count(parse_frame, frame_count).parse(i)
}

pub fn parse_spr(i: &'_ [u8]) -> IResult<'_, Spr> {
    let (i, header) = parse_header.parse(i)?;
    let (i, palette) = parse_palette(i, header.palette_count as usize)?;
    let (i, frames) = parse_frames(i, header.frame_num as usize)?;

    Ok((
        i,
        Spr {
            header,
            palette,
            frames,
        },
    ))
}
