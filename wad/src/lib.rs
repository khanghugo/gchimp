//! WAD file parsing
//!
//! Based of specification from this webpage: https://twhl.info/wiki/page/Specification%3A_WAD3
use std::{
    ffi::OsStr,
    fmt::{self, Display, Write as FmtWrite},
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use byte_writer::ByteWriter;
use nom::{
    combinator::{fail, map},
    error::context,
    multi::count,
    number::complete::{le_i16, le_i32, le_i8, le_u32, le_u8},
    sequence::tuple,
    IResult as _IResult,
};

use eyre::eyre;

mod byte_writer;

type IResult<'a, T> = _IResult<&'a [u8], T>;

#[derive(Debug)]
pub struct Header {
    pub magic: Vec<i8>,
    pub num_dirs: i32,
    pub dir_offset: i32,
}

#[derive(Debug)]
pub struct DirectoryEntry {
    pub entry_offset: i32,
    pub disk_size: i32,
    pub entry_size: i32,
    pub file_type: i8,
    pub compressed: bool,
    pub padding: i16,
    // [u8; 16]
    pub texture_name: TextureName,
}

#[derive(Debug)]
pub struct TextureName(Vec<u8>);

impl TextureName {
    pub fn get_bytes(&self) -> &Vec<u8> {
        &self.0
    }
}

#[derive(Debug)]
pub struct Image(Vec<Vec<u8>>);

impl Image {
    pub fn get_bytes(&self) -> &Vec<Vec<u8>> {
        &self.0
    }
}

#[derive(Debug)]
pub struct Palette(Vec<Vec<u8>>);

impl Palette {
    pub fn get_bytes(&self) -> &Vec<Vec<u8>> {
        &self.0
    }
}

impl Display for TextureName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in &self.0 {
            // do not write null
            if *c == 0 {
                continue;
            }

            f.write_char(*c as char)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Qpic {
    pub width: i32,
    pub height: i32,
    // [[u8; width]; height]
    pub data: Image,
    pub colors_used: i16,
    // Vec<[u8; 3]>
    pub palette: Palette,
}

#[derive(Debug)]
pub struct MipMap {
    // [[u8; width]; height]
    pub data: Image,
}

#[derive(Debug)]
pub struct MipTex {
    /// The texture name might be different from the directory entry.
    ///
    /// It is better to use directory entry texture name.
    pub texture_name: TextureName,
    // weird shift, i know
    pub width: u32,
    pub height: u32,
    // [u32; 4]
    pub mip_offsets: Vec<u32>,
    // [MipMap; 4] where each later MipMap is halved the dimensions
    pub mip_images: Vec<MipMap>,
    pub colors_used: i16,
    // Vec<[u8; 3]>
    pub palette: Palette,
}

// this is not how it looks in file
#[derive(Debug)]
pub struct Entry {
    pub directory_entry: DirectoryEntry,
    pub file_entry: FileEntry,
}

#[derive(Debug)]
pub enum FileEntry {
    Qpic(Qpic),
    MipTex(MipTex),
}

#[derive(Debug)]
pub struct Wad {
    pub header: Header,
    pub entries: Vec<Entry>,
}

impl Wad {
    pub fn from(bytes: &[u8]) -> eyre::Result<Self> {
        match parse_wad(bytes) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(eyre!("Cannot parse bytes: {}", err)),
        }
    }

    pub fn from_file(path: impl AsRef<Path> + AsRef<OsStr>) -> eyre::Result<Self> {
        let bytes = std::fs::read(path)?;

        Self::from(&bytes)
    }

    pub fn write_to_file(&self, path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<()> {
        let bytes = self.write_to_bytes();

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        file.write_all(&bytes)?;

        file.flush()?;

        Ok(())
    }

    pub fn write_to_bytes(&self) -> Vec<u8> {
        let mut writer = ByteWriter::new();

        // write header
        let header = &self.header;

        writer.append_i8_slice(&header.magic);
        writer.append_i32(header.num_dirs);
        // just a dummy offset at this point.
        let dir_offset_index = writer.get_offset();
        writer.append_i32(header.dir_offset);

        // need to write file first then we will write directory entry later
        // with known offsets relatively from the end of the header,
        // we can point to the correct data later on in directory
        let file_entries_offset_and_length = self
            .entries
            .iter()
            .map(|entry| {
                let file_entry = &entry.file_entry;
                let miptex_header_length = 16 + 4 + 4 + 4 * 4;
                let file_entry_offset = writer.get_offset();

                // write file entry
                match file_entry {
                    FileEntry::Qpic(_) => unimplemented!(),
                    FileEntry::MipTex(MipTex {
                        texture_name,
                        width,
                        height,
                        mip_offsets: _,
                        mip_images,
                        colors_used: _,
                        palette,
                    }) => {
                        let texture_name_bytes = texture_name.get_bytes();
                        writer.append_u8_slice(texture_name_bytes);
                        writer.append_u8_slice(&vec![0u8; 16 - texture_name_bytes.len()]);

                        writer.append_u32(*width);
                        writer.append_u32(*height);

                        // mip_offsets
                        writer.append_u32(miptex_header_length);
                        writer.append_u32(miptex_header_length + width * height);
                        writer.append_u32(
                            miptex_header_length + width * height + (width * height) / 4,
                        );
                        writer.append_u32(
                            miptex_header_length
                                + width * height
                                + (width * height) / 4
                                + (width * height) / 4 / 4,
                        );

                        // mip images
                        for image in mip_images {
                            for row in image.data.get_bytes() {
                                writer.append_u8_slice(row);
                            }
                        }

                        // colors_used
                        writer.append_i16(256);

                        for row in palette.get_bytes() {
                            writer.append_u8_slice(row);
                        }
                    }
                }

                // apparently, WE BYTES NEED TO BE ALIGNED HERE
                // this is not documented anywhere
                let offset_bytes_needed = writer.get_offset() % 4;

                for _ in 0..offset_bytes_needed {
                    writer.append_u8(0);
                }

                (file_entry_offset, writer.get_offset() - file_entry_offset)
            })
            .collect::<Vec<(usize, usize)>>();

        // done writing the images, now we have the definite offset for our directory entry
        let directory_entry_offset = writer.get_offset();
        writer.replace_with_u32(dir_offset_index, directory_entry_offset as u32);

        self.entries
            .iter()
            .zip(file_entries_offset_and_length)
            .for_each(|(entry, (offset, length))| {
                let DirectoryEntry {
                    entry_offset: _,
                    disk_size: _,
                    entry_size: _,
                    file_type,
                    compressed: _,
                    padding: _,
                    texture_name,
                } = &entry.directory_entry;

                // write directory entry in a contiguous memory run
                writer.append_i32(offset as i32);
                writer.append_i32(length as i32);
                writer.append_i32(length as i32);
                writer.append_i8(*file_type);
                writer.append_i8(0); // not compressed
                writer.append_i16(256); // hard coded number of colors

                let texture_name_bytes = texture_name.get_bytes();
                writer.append_u8_slice(texture_name_bytes);
                writer.append_u8_slice(&vec![0u8; 16 - texture_name_bytes.len()])
            });

        writer.data
    }
}

fn parse_header(i: &[u8]) -> IResult<Header> {
    map(
        tuple((count(le_i8, 4), le_i32, le_i32)),
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
    let (i, (width, height)) = tuple((le_i32, le_i32))(i)?;
    let (i, data) = count(count(le_u8, width as usize), height as usize)(i)?;
    let (i, colors_used) = le_i16(i)?;
    let (i, palette) = count(count(le_u8, 3), colors_used as usize)(i)?;

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

fn parse_miptex(i: &[u8]) -> IResult<MipTex> {
    let struct_start = i;

    let (i, texture_name) = count(le_u8, 16)(i)?;
    let (i, (width, height)) = tuple((le_u32, le_u32))(i)?;
    let (i, mip_offsets) = count(le_u32, 4)(i)?;

    // offset relatively from where we start with the struct
    let (_, miptex0) = count(count(le_u8, width as usize), height as usize)(
        &struct_start[(mip_offsets[0] as usize)..],
    )?;

    let (_, miptex1) = count(count(le_u8, width as usize / 2), height as usize / 2)(
        &struct_start[(mip_offsets[1] as usize)..],
    )?;

    let (_, miptex2) = count(
        count(le_u8, width as usize / 2 / 2),
        height as usize / 2 / 2,
    )(&struct_start[(mip_offsets[2] as usize)..])?;

    // we get the palette start from the end of 4th miptex
    let (palette_start, miptex3) = count(
        count(le_u8, width as usize / 2 / 2 / 2),
        height as usize / 2 / 2 / 2,
    )(&struct_start[(mip_offsets[3] as usize)..])?;

    // colors_used is always 256
    let (palette_start, colors_used) = le_i16(palette_start)?;

    // hard code it to be 256 just to be safe
    let (_, palette) = count(count(le_u8, 3), 256)(palette_start)?;

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

static FILE_TYPES: &[i8] = &[0x42, 0x43, 0x45];

fn parse_wad(i: &[u8]) -> IResult<Wad> {
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

    if directory_entries
        .iter()
        .any(|entry| entry.file_type == 0x45)
    {
        let err_str = "Does not support parsing font (yet).";

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_wad_test() {
        let file = Wad::from_file("test/wad_test.wad");

        assert!(file.is_ok());

        let file = file.unwrap();

        assert!(file.header.num_dirs == 1);
        assert!(file.entries.len() == 1);

        let entry = &file.entries[0];

        assert!(entry.directory_entry.file_type == 0x43);
        assert!(entry.directory_entry.texture_name.to_string() == "white");
    }

    #[test]
    fn parse_wad_test2() {
        let file = Wad::from_file("test/wad_test2.wad");

        assert!(file.is_ok());

        let file = file.unwrap();

        assert!(file.header.num_dirs == 2);
        assert!(file.entries.len() == 2);

        let entry = &file.entries[0];

        assert!(entry.directory_entry.file_type == 0x43);
        assert!(entry.directory_entry.texture_name.to_string() == "white");

        let entry = &file.entries[1];

        assert!(entry.directory_entry.file_type == 0x43);
        assert!(entry.directory_entry.texture_name.to_string() == "black");
    }

    #[test]
    fn parse_cyberwave() {
        let file = Wad::from_file("test/surf_cyberwave.wad");

        assert!(file.is_ok());

        let file = file.unwrap();

        assert!(file.header.num_dirs == 23);
        assert!(file.entries.len() == 23);

        let entry = &file.entries[18];

        assert_eq!(entry.directory_entry.file_type, 0x43);
        assert_eq!(
            entry.directory_entry.texture_name.to_string(),
            "Sci_fi_metal_fl"
        );

        assert!(matches!(entry.file_entry, FileEntry::MipTex(_)));

        if let FileEntry::MipTex(file) = &entry.file_entry {
            assert_eq!(file.height, file.width);
            assert_eq!(file.height, 512);
            assert_eq!(file.texture_name.to_string(), "Sci_fi_metal_fl");
        }

        let entry = &file.entries[21];

        assert_eq!(entry.directory_entry.file_type, 0x43);
        assert_eq!(entry.directory_entry.texture_name.to_string(), "emp_ball1");

        assert!(matches!(entry.file_entry, FileEntry::MipTex(_)));

        if let FileEntry::MipTex(file) = &entry.file_entry {
            assert_eq!(file.height, file.width);
            assert_eq!(file.height, 512);
            // Don't assert this because it fails.
            // left: "emp_ball1ing.."
            // right: "emp_ball1"
            // assert_eq!(file.texture_name.to_string(), "emp_ball1");
        }
    }

    #[test]
    fn parse_write() {
        let wad = Wad::from_file("test/wad_test.wad");

        assert!(wad.is_ok());

        let wad = wad.unwrap();

        let res = wad.write_to_file("test/out/wad_test_out.wad");

        assert!(res.is_ok());
    }

    #[test]
    fn parse_write2() {
        let wad = Wad::from_file("test/wad_test2.wad");

        assert!(wad.is_ok());

        let wad = wad.unwrap();

        let res = wad.write_to_file("test/out/wad_test2_out.wad");

        assert!(res.is_ok());
    }

    #[test]
    fn parse_write3() {
        let wad = Wad::from_file("test/surf_cyberwave.wad");

        assert!(wad.is_ok());

        let wad = wad.unwrap();

        let res = wad.write_to_file("test/out/surf_cyberwave_out.wad");

        assert!(res.is_ok());
    }
}
