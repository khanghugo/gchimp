use eyre::eyre;
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
    // UPDATE: i did not hardcode and this happens
    // this is just to make sure that wally does not do dumb shit to normal wads again
    let colors_used = if colors_used <= 0 || colors_used > 256 {
        256
    } else {
        colors_used
    };

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
        map(
            tuple((le_i8, le_i8, le_i16)),
            |(offset_y, offset_x, charwidth)| CharInfo {
                offset_y,
                offset_x,
                charwidth,
            },
        ),
        256,
    )(i)?;

    let (i, data) = count(le_u8, (width * height) as usize)(i)?;
    let (i, colors_used) = le_i16(i)?;

    // println!("color used is {}", colors_used);
    // color used is always 256 because of course why not.

    let (i, palette) = count(
        map(take(3usize), |res: &[u8]| [res[0], res[1], res[2]]),
        // colors_used as usize,
        256,
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

static FILE_TYPES: &[i8] = &[0x40, 0x42, 0x43, 0x45, 0x46];

pub fn parse_wad(i: &[u8]) -> IResult<Wad> {
    let file_start = i;

    let (_, header) = parse_header(i)?;

    if header.magic != "WAD3".as_bytes() {
        return context("wad file is not WAD3", fail)(&[]);
    }

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

    if let Some(unknown_file_entry) = directory_entries
        .iter()
        .find(|entry| !FILE_TYPES.contains(&entry.file_type))
    {
        let err_str = format!(
            "unknown texture file type: {:#02x}",
            unknown_file_entry.file_type
        )
        .leak();

        println!("{}", err_str);

        return context(err_str, fail)(b"");
    };

    let file_entries = directory_entries
        .iter()
        .enumerate()
        .map(|(entry_index, directory_entry)| {
            // the actual WAD data is from the beginning of the file, not the beginning of the directory entry
            let file_entry_start = &file_start[directory_entry.entry_offset as usize..];

            match directory_entry.file_type {
                0x42 => {
                    let Ok((_, res)) = parse_qpic(file_entry_start) else {
                        return Err(eyre!("cannot parse qpic (entry {entry_index})"));
                    };

                    Ok(FileEntry::Qpic(res))
                }
                0x43 | 0x40 => {
                    let Ok((_, res)) = parse_miptex(file_entry_start) else {
                        return Err(eyre!("cannot parse miptex (entry {entry_index})"));
                    };

                    Ok(FileEntry::MipTex(res))
                }
                0x45 | 0x46 => {
                    let Ok((_, res)) = parse_font(file_entry_start) else {
                        return Err(eyre!("cannot parse font (entry {entry_index})"));
                    };
                    Ok(FileEntry::Font(res))
                }
                _ => unreachable!(""),
            }
        })
        .collect::<Vec<eyre::Result<FileEntry>>>();

    let err_str = file_entries
        .iter()
        .filter_map(|e| e.as_ref().err())
        .fold(String::new(), |acc, e| format!("{acc}{e}\n"));

    // second clause is just to make sure
    if !err_str.is_empty() && file_entries.iter().any(|e| e.is_err()) {
        return context(err_str.leak(), fail)(b"");
    }

    let file_entries: Vec<FileEntry> = file_entries.into_iter().filter_map(|e| e.ok()).collect();

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
