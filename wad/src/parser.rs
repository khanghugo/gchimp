use nom::{
    bytes::complete::take,
    combinator::{fail, map},
    error::context,
    multi::count,
    number::complete::{le_i16, le_i32, le_i8, le_u32, le_u8},
    sequence::tuple,
    IResult as _IResult,
};

use crate::types::{
    CharInfo, DirectoryEntry, Entry, FileEntry, Font, Header, Image, MipMap, MipTex, Palette, Qpic,
    TextureName, Wad,
};

type IResult<'a, T> = _IResult<&'a [u8], T>;

fn parse_header(i: &[u8]) -> IResult<Header> {
    map(
        tuple((count(le_u8, 4), le_i32, le_i32)),
        |(magic, num_dirs, dir_offset)| Header {
            magic,
            num_dirs,
            dir_offset,
        },
    )(i)
}

fn parse_directory_entry(i: &[u8]) -> IResult<DirectoryEntry> {
    map(
        tuple((
            le_i32,
            le_i32,
            le_i32,
            le_i8,
            le_i8, // https://github.com/Ty-Matthews-VisualStudio/Wally/blob/a05d3a11ac69aa81725fc7d4c6497b0523e92657/Source/Wally/WADList.h#L36
            le_i16,
            count(le_u8, 16),
        )),
        |(entry_offset, disk_size, entry_size, file_type, compressed, padding, texture_name)| {
            DirectoryEntry {
                entry_offset,
                disk_size,
                entry_size,
                file_type,
                compressed: compressed != 0,
                padding,
                texture_name: TextureName(texture_name),
            }
        },
    )(i)
}

fn parse_qpic(i: &[u8]) -> IResult<Qpic> {
    let (i, (width, height)) = tuple((le_u32, le_u32))(i)?;
    let (i, data) = count(le_u8, (width * height) as usize)(i)?;
    let (i, colors_used) = le_i16(i)?;
    let (i, palette) = count(
        map(take(3usize), |res: &[u8]| [res[0], res[1], res[2]]),
        colors_used as usize,
    )(i)?;

    Ok((
        i,
        Qpic {
            width,
            height,
            data: Image(data),
            colors_used,
            palette: Palette(palette),
        },
    ))
}

pub fn parse_miptex(i: &[u8]) -> IResult<MipTex> {
    let struct_start = i;

    let (i, texture_name) = count(le_u8, 16)(i)?;
    let (i, (width, height)) = tuple((le_u32, le_u32))(i)?;
    let (i, mip_offsets) = count(le_u32, 4)(i)?;

    if mip_offsets[0] == 0 {
        return Ok((
            i,
            MipTex {
                texture_name: TextureName(texture_name),
                width,
                height,
                mip_offsets,
                mip_images: vec![],
                colors_used: 0,
                palette: Palette(vec![]),
            },
        ));
    }

    // offset relatively from where we start with the struct
    let (_, miptex0) =
        count(le_u8, (width * height) as usize)(&struct_start[(mip_offsets[0] as usize)..])?;

    let (_, miptex1) =
        count(le_u8, (width * height / 4) as usize)(&struct_start[(mip_offsets[1] as usize)..])?;

    let (_, miptex2) = count(le_u8, (width * height / 4 / 4) as usize)(
        &struct_start[(mip_offsets[2] as usize)..],
    )?;

    // we get the palette start from the end of 4th miptex
    let (palette_start, miptex3) = count(le_u8, (width * height / 4 / 4 / 4) as usize)(
        &struct_start[(mip_offsets[3] as usize)..],
    )?;

    // colors_used is always 256
    let (palette_start, colors_used) = le_i16(palette_start)?;

    // hard code it to be 256 just to be safe
    let (_, palette) = count(
        map(take(3usize), |res: &[u8]| [res[0], res[1], res[2]]),
        colors_used as usize,
    )(palette_start)?;

    Ok((
        i, // i here is pretty useless
        MipTex {
            texture_name: TextureName(texture_name),
            width,
            height,
            mip_offsets,
            mip_images: vec![
                MipMap {
                    data: Image(miptex0),
                },
                MipMap {
                    data: Image(miptex1),
                },
                MipMap {
                    data: Image(miptex2),
                },
                MipMap {
                    data: Image(miptex3),
                },
            ],
            colors_used,
            palette: Palette(palette),
        },
    ))
}

fn parse_font(i: &[u8]) -> IResult<Font> {
    let (i, (width, height)) = tuple((le_u32, le_u32))(i)?;
    let (i, (row_count, row_height)) = tuple((le_u32, le_u32))(i)?;

    let (i, font_info) = count(
        map(tuple((le_i16, le_i16)), |(startoffset, charwidth)| {
            CharInfo {
                startoffset,
                charwidth,
            }
        }),
        256,
    )(i)?;

    let (i, data) = count(le_u8, (width * height) as usize)(i)?;
    let (i, colors_used) = le_i16(i)?;
    let (i, palette) = count(
        map(take(3usize), |res: &[u8]| [res[0], res[1], res[2]]),
        colors_used as usize,
    )(i)?;

    Ok((
        i,
        Font {
            width,
            height,
            row_count,
            row_height,
            font_info,
            data: Image(data),
            colors_used,
            palette: Palette(palette),
        },
    ))
}

static FILE_TYPES: &[i8] = &[0x42, 0x43, 0x45];

pub fn parse_wad(i: &[u8]) -> IResult<Wad> {
    let file_start = i;

    let (_, header) = parse_header(i)?;

    let dir_start = &i[(header.dir_offset as usize)..];
    let (_, directory_entries) = count(parse_directory_entry, header.num_dirs as usize)(dir_start)?;

    if directory_entries.len() != header.num_dirs as usize {
        let err_str = "Mismatched number of entries in header and number of parsed entries.";

        println!("{}", err_str);

        return context(err_str, fail)(b"");
    }

    if directory_entries.iter().any(|entry| entry.compressed) {
        let err_str = "Does not support parsing compressed textures.";

        println!("{}", err_str);

        return context(err_str, fail)(b"");
    }

    if directory_entries
        .iter()
        .any(|entry| !FILE_TYPES.contains(&entry.file_type))
    {
        let err_str = "Unknown texture file type.";

        println!("{}", err_str);

        return context(err_str, fail)(b"");
    };

    let file_entries = directory_entries
        .iter()
        .filter_map(|directory_entry| {
            // the actual WAD data is from the beginning of the file, not the beginning of the directory entry
            let file_entry_start = &file_start[directory_entry.entry_offset as usize..];

            let file_entry = match directory_entry.file_type {
                0x42 => FileEntry::Qpic(parse_qpic(file_entry_start).ok()?.1),
                0x43 => FileEntry::MipTex(parse_miptex(file_entry_start).ok()?.1),
                0x45 => FileEntry::Font(parse_font(file_entry_start).ok()?.1),
                _ => unreachable!(""),
            };

            Some(file_entry)
        })
        .collect::<Vec<FileEntry>>();

    if file_entries.len() != directory_entries.len() {
        let err_str = "Failed parsing texture data.";

        println!("{}", err_str);

        return context(err_str, fail)(b"");
    }

    let entries = directory_entries
        .into_iter()
        .zip(file_entries)
        .map(|(directory, file)| Entry {
            directory_entry: directory,
            file_entry: file,
        })
        .collect::<Vec<Entry>>();

    Ok((
        i, // this is useless
        Wad { header, entries },
    ))
}
