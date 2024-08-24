use nom::{
    bytes::complete::take,
    combinator::{fail, map},
    error::context,
    multi::count,
    number::complete::{le_f32, le_i16, le_i32, le_u16, le_u32, le_u8},
    sequence::tuple,
};

use crate::{
    formats::{VtfImage, VtfImageFormat},
    Face, Frame, Header, Header72, Header73, IResult, MipMap, Resource, ResourceEntry,
    ResourceEntryTag, Vtf, Vtf72Data, Vtf73Data, VtfData, VtfFlag, VtfHighResImage,
};

fn parse_header(i: &[u8]) -> IResult<Header> {
    let (i, signature) = take(4usize)(i)?;

    let (i, (version, header_size, width, height, flags, frames, first_frame)) = tuple((
        count(le_u32, 2),
        le_u32,
        le_u16,
        le_u16,
        le_u32,
        le_u16,
        le_i16,
    ))(i)?;

    if version[0] != 7 {
        return context("VTF version is not 7", fail)(i);
    }

    let (i, _) = take(4usize)(i)?;
    let (i, reflectivity) = count(le_f32, 3)(i)?;
    let (i, _) = take(4usize)(i)?;

    let (
        i,
        (
            bump_map_scale,
            high_res_image_format,
            mipmap_count,
            low_res_image_format,
            low_res_image_width,
            low_res_image_height,
        ),
    ) = tuple((le_f32, le_i32, le_u8, le_i32, le_u8, le_u8))(i)?;

    let (i, header72) = if version[1] >= 2 {
        let (i, depth) = le_u16(i)?;

        (i, Some(Header72 { depth }))
    } else {
        (i, None)
    };

    let (i, header73) = if version[1] >= 3 {
        let (i, _) = take(3usize)(i)?;
        let (i, num_resources) = le_u32(i)?;
        let (i, _) = take(8usize)(i)?;

        (i, Some(Header73 { num_resources }))
    } else {
        (i, None)
    };

    Ok((
        i,
        Header {
            signature: signature.to_vec(),
            version,
            header_size,
            width,
            height,
            flags,
            frames,
            first_frame,
            reflectivity,
            bump_map_scale,
            high_res_image_format,
            mipmap_count,
            low_res_image_format,
            low_res_image_width,
            low_res_image_height,
            header72,
            header73,
        },
    ))
}

fn parse_vtf72_data(i: &[u8]) -> IResult<Vtf72Data> {
    todo!()
}

fn parse_vtf73_data<'a, 'b>(
    i: &'a [u8],
    header_end: &'a [u8],
    header: &'b Header,
) -> IResult<'a, Vtf73Data> {
    // i is the beginning of the file
    let (_, entries) = count(
        parse_resource_entry,
        header.header73.as_ref().unwrap().num_resources as usize,
    )(header_end)
    .unwrap();

    let mut res: Vec<Resource> = vec![];

    for entry in entries {
        let i = &i[(entry.offset as usize)..];

        match entry.tag {
            ResourceEntryTag::LowRes => {
                let format = VtfImageFormat::try_from(header.low_res_image_format);

                if let Err(err) = format {
                    return context(err, fail)(i);
                }

                let format = format.unwrap();

                let (_, image) = VtfImage::parse_from_format(
                    i,
                    format,
                    (
                        header.low_res_image_width as u32,
                        header.low_res_image_height as u32,
                    ),
                )?;

                res.push(Resource::LowRes(image));
            }
            ResourceEntryTag::HighRes => {
                let (_, mipmaps) = parse_high_res_mipmaps(i, header)?;

                res.push(Resource::HighRes(VtfHighResImage { mipmaps }));
            }
            ResourceEntryTag::AnimatedParticleSheet => todo!(),
            ResourceEntryTag::CRC => todo!(),
            ResourceEntryTag::TextureLODControl => todo!(),
            ResourceEntryTag::ExtendedVTF => todo!(),
            ResourceEntryTag::KeyValues => todo!(),
        }
    }

    Ok((i, res))
}

// TODO: refactor this to just map(count, x)
fn parse_high_res_mipmaps<'a, 'b>(i: &'a [u8], header: &'b Header) -> IResult<'a, Vec<MipMap>> {
    let format = VtfImageFormat::try_from(header.high_res_image_format);

    if let Err(err) = format {
        return context(err, fail)(i);
    }

    let format = format.unwrap();

    let mut i = i;

    let mut mipmaps: Vec<MipMap> = vec![];
    for mipmap_idx in 0..(header.mipmap_count as usize) {
        // mipmaps are sorted from smallest to biggest
        // mipmaps map dimensions are halved every time
        let scalar = 2u16.pow((header.mipmap_count as usize - (mipmap_idx + 1)) as u32);
        let (width, height) = (header.width / scalar, header.height / scalar);

        let mut frames: Vec<Frame> = vec![];
        for _frame_idx in 0..(header.frames as usize) {
            let face_count = if header.flags & VtfFlag::TextureflagsEnvmap as u32 == 1 {
                if header.version[2] < 5 && header.first_frame == -1 {
                    7
                } else {
                    6
                }
            } else {
                1
            };

            let mut faces: Vec<Face> = vec![];
            for _face_idx in 0..(face_count as usize) {
                let (new_i, image) =
                    VtfImage::parse_from_format(i, format, (width as u32, height as u32))?;

                i = new_i;
                faces.push(Face { image });
            }

            frames.push(Frame { faces })
        }

        mipmaps.push(MipMap { frames })
    }

    Ok((i, mipmaps))
}

// 7.3+
fn parse_resource_entry(i: &[u8]) -> IResult<ResourceEntry> {
    let (i, (tag, flags, offset)) = tuple((take(3usize), le_u8, le_u32))(i)?;

    let tag_res = ResourceEntryTag::try_from(tag);

    if let Err(err) = tag_res {
        return context(err, fail)(i);
    }

    return Ok((
        i,
        ResourceEntry {
            tag: tag_res.unwrap(),
            flags,
            offset,
        },
    ));
}

pub fn parse_vtf(i: &[u8]) -> IResult<Vtf> {
    let (header_end, header) = parse_header(i)?;

    if header.version[0] != 7 {
        return context("VTF major version is not 7", fail)(b"");
    }

    let data = if header.version[1] >= 3 {
        let (_, data) = parse_vtf73_data(i, header_end, &header)?;
        VtfData::Vtf73(data)
    } else if header.version[1] == 2 {
        let (_, data) = parse_vtf72_data(i)?;
        VtfData::Vtf72(data)
    } else {
        unreachable!("VTF minor version {} is not supported", header.version[1])
    };

    Ok((b"", Vtf { header, data }))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn header() {
        let vtf = include_bytes!("tests/tilefloor01.vtf");
        let (end_header, header) = parse_header(vtf).unwrap();

        println!("{:?}", header);
    }

    #[test]
    fn vtf() {
        let vtf = include_bytes!("tests/tilefloor01.vtf");
        let (_, vtf) = parse_vtf(vtf).unwrap();
        println!("vtf {:?}", vtf);
    }
}
