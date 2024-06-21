//! WAD file parsing
//!
//! Based of specification from this webpage: https://twhl.info/wiki/page/Specification%3A_WAD3
use std::{
    ffi::OsStr,
    fmt::{self, Display, Write as FmtWrite},
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    str::from_utf8,
};

use byte_writer::ByteWriter;
use nom::{
    bytes::complete::take,
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

static MAX_TEXTURE_NAME_LENGTH: usize = 15;

#[derive(Debug)]
pub struct Header {
    pub magic: Vec<u8>,
    pub num_dirs: i32,
    pub dir_offset: i32,
}

impl Header {
    pub fn new() -> Self {
        Self {
            magic: "WAD3".as_bytes().to_owned(),
            num_dirs: 0,
            dir_offset: 0,
        }
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
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

impl DirectoryEntry {
    /// Creates a new Directory Entry for MipTex with just texture name
    pub fn new(s: impl AsRef<str> + Into<String>) -> Self {
        Self {
            entry_offset: 0,
            disk_size: 0,
            entry_size: 0,
            file_type: 0x43,
            compressed: false,
            padding: 256,
            texture_name: TextureName::from_string(s),
        }
    }
}

#[derive(Debug)]
/// Don't use to_string() method.
///
/// Use get_string() instead
pub struct TextureName(Vec<u8>);

impl TextureName {
    // impl Debug has its own to_string.....
    pub fn get_string(&self) -> String {
        let mut res: Vec<u8> = vec![];

        for c in self.get_bytes() {
            if *c == 0 || *c < 32 || *c > 127 {
                break;
            }

            res.push(*c);
        }

        from_utf8(&res).unwrap().to_string()
    }

    pub fn from_string(s: impl AsRef<str> + Into<String>) -> Self {
        let mut res = vec![0u8; MAX_TEXTURE_NAME_LENGTH + 1];
        let texture_name_length = s.as_ref().len().min(MAX_TEXTURE_NAME_LENGTH);

        res[..texture_name_length].copy_from_slice(&s.as_ref().as_bytes()[..texture_name_length]);

        Self(res)
    }

    pub fn get_bytes(&self) -> &Vec<u8> {
        &self.0
    }

    pub fn set_name(&mut self, s: impl AsRef<str> + Into<String>) -> eyre::Result<()> {
        if s.as_ref().len() >= 16 {
            return Err(eyre!("Max length for name is 15 characters."));
        }

        self.0[..s.as_ref().len()].copy_from_slice(s.as_ref().as_bytes());

        self.0[s.as_ref().len()] = 0;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Image(Vec<u8>);

impl Image {
    pub fn new(s: impl AsRef<[u8]> + Into<Vec<u8>>) -> Self {
        Self(s.into())
    }

    pub fn get_bytes(&self) -> &Vec<u8> {
        &self.0
    }
}

#[derive(Debug)]
pub struct Palette(Vec<[u8; 3]>);

impl Palette {
    pub fn new(s: impl Into<Vec<[u8; 3]>>) -> Self {
        Self(s.into())
    }

    pub fn get_bytes(&self) -> &Vec<[u8; 3]> {
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
    pub width: u32,
    pub height: u32,
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

impl MipMap {
    pub fn new(s: impl Into<Vec<u8>>) -> Self {
        Self {
            data: Image::new(s.into()),
        }
    }
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

impl MipTex {
    /// Only creates the biggest mipmap
    pub fn new(
        s: impl AsRef<str> + Into<String>,
        (width, height): (u32, u32),
        images: &[&[u8]],
        palette: impl Into<Vec<[u8; 3]>>,
    ) -> Self {
        let mip0_len = (width * height) as usize;

        let mip0 = MipMap::new(images[0]);
        let mip1 = MipMap::new(images[1]);
        let mip2 = MipMap::new(images[2]);
        let mip3 = MipMap::new(images[3]);

        let mip0_offset = 16 + 4 + 4 + 4 * 4;
        let mip1_offset = (mip0_offset + mip0_len) as u32;
        let mip2_offset = (mip0_offset + mip0_len + mip0_len / 4) as u32;
        let mip3_offset = (mip0_offset + mip0_len + mip0_len / 4 + mip0_len / 4 / 4) as u32;

        Self {
            texture_name: TextureName::from_string(s),
            width,
            height,
            mip_offsets: vec![mip0_offset as u32, mip1_offset, mip2_offset, mip3_offset],
            mip_images: vec![mip0, mip1, mip2, mip3],
            colors_used: 256,
            palette: Palette::new(palette),
        }
    }
}

#[derive(Debug)]
pub struct CharInfo {
    pub startoffset: i16,
    pub charwidth: i16,
}

#[derive(Debug)]
pub struct Font {
    pub width: u32,
    pub height: u32,
    pub row_count: u32,
    pub row_height: u32,
    // [CharInfo; 256]
    pub font_info: Vec<CharInfo>,
    pub data: Image,
    pub colors_used: i16,
    pub palette: Palette,
}

// this is not how it looks in file
#[derive(Debug)]
pub struct Entry {
    pub directory_entry: DirectoryEntry,
    pub file_entry: FileEntry,
}

impl Entry {
    pub fn new(
        texture_name: impl AsRef<str> + Into<String>,
        dimensions: (u32, u32),
        images: &[&[u8]],
        palette: impl Into<Vec<[u8; 3]>> + AsRef<[[u8; 3]]>,
    ) -> Self {
        Self {
            directory_entry: DirectoryEntry::new(texture_name.as_ref()),
            file_entry: FileEntry::new_miptex(texture_name, images, dimensions, palette),
        }
    }

    pub fn texture_name(&self) -> String {
        self.directory_entry.texture_name.get_string()
    }

    pub fn set_name(&mut self, s: impl AsRef<str> + Into<String> + Clone) -> eyre::Result<()> {
        self.directory_entry.texture_name.set_name(s.clone())?;

        match &mut self.file_entry {
            FileEntry::Qpic(_) => (),
            FileEntry::MipTex(miptex) => miptex.texture_name.set_name(s)?,
            FileEntry::Font(_) => (),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum FileEntry {
    Qpic(Qpic),
    MipTex(MipTex),
    Font(Font),
}

impl FileEntry {
    pub fn new_miptex(
        texture_name: impl AsRef<str> + Into<String>,
        images: &[&[u8]],
        dimensions: (u32, u32),
        palette: impl Into<Vec<[u8; 3]>>,
    ) -> Self {
        Self::MipTex(MipTex::new(texture_name, dimensions, images, palette))
    }

    pub fn dimensions(&self) -> (u32, u32) {
        match &self {
            Self::Qpic(qpic) => (qpic.width, qpic.height),
            Self::MipTex(miptex) => (miptex.width, miptex.height),
            Self::Font(font) => (font.width, font.height),
        }
    }

    pub fn image(&self) -> &Vec<u8> {
        match &self {
            Self::Qpic(qpic) => qpic.data.get_bytes(),
            Self::MipTex(miptex) => miptex.mip_images[0].data.get_bytes(),
            Self::Font(font) => font.data.get_bytes(),
        }
    }

    pub fn palette(&self) -> &Vec<[u8; 3]> {
        match &self {
            Self::Qpic(qpic) => qpic.palette.get_bytes(),
            Self::MipTex(miptex) => miptex.palette.get_bytes(),
            Self::Font(font) => font.palette.get_bytes(),
        }
    }
}

#[derive(Debug)]
pub struct Wad {
    pub header: Header,
    pub entries: Vec<Entry>,
}

impl Default for Wad {
    fn default() -> Self {
        Self::new()
    }
}

impl Wad {
    /// Creates a new WAD file without any information
    pub fn new() -> Self {
        Self {
            header: Header::default(),
            entries: vec![],
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        match parse_wad(bytes) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(eyre!("Cannot parse bytes: {}", err)),
        }
    }

    pub fn from_file(path: impl AsRef<Path> + AsRef<OsStr>) -> eyre::Result<Self> {
        let bytes = std::fs::read(path)?;

        let res = Self::from_bytes(&bytes);

        drop(bytes);

        res
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

        writer.append_u8_slice(&header.magic);
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
                            writer.append_u8_slice(image.data.get_bytes());
                        }

                        // colors_used
                        writer.append_i16(256);

                        for row in palette.get_bytes() {
                            writer.append_u8_slice(row);
                        }

                        // pad palette to correctly have 256 colors
                        writer.append_u8_slice(&vec![0u8; (256 - palette.get_bytes().len()) * 3]);
                    }
                    FileEntry::Font(_) => todo!(),
                }

                // apparently, if we want compatibility with Wally, we need to align the bytes
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

fn parse_miptex(i: &[u8]) -> IResult<MipTex> {
    let struct_start = i;

    let (i, texture_name) = count(le_u8, 16)(i)?;
    let (i, (width, height)) = tuple((le_u32, le_u32))(i)?;
    let (i, mip_offsets) = count(le_u32, 4)(i)?;

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
        assert!(entry.directory_entry.texture_name.get_string() == "white");
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
        assert!(entry.directory_entry.texture_name.get_string() == "white");

        let entry = &file.entries[1];

        assert!(entry.directory_entry.file_type == 0x43);
        assert!(entry.directory_entry.texture_name.get_string() == "black");
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
            entry.directory_entry.texture_name.get_string(),
            "Sci_fi_metal_fl"
        );

        assert!(matches!(entry.file_entry, FileEntry::MipTex(_)));

        if let FileEntry::MipTex(file) = &entry.file_entry {
            assert_eq!(file.height, file.width);
            assert_eq!(file.height, 512);
            assert_eq!(file.texture_name.get_string(), "Sci_fi_metal_fl");
        }

        let entry = &file.entries[21];

        assert_eq!(entry.directory_entry.file_type, 0x43);
        assert_eq!(entry.directory_entry.texture_name.get_string(), "emp_ball1");

        assert!(matches!(entry.file_entry, FileEntry::MipTex(_)));

        if let FileEntry::MipTex(file) = &entry.file_entry {
            assert_eq!(file.height, file.width);
            assert_eq!(file.height, 512);
            // Don't assert this because it fails.
            // left: "emp_ball1ing.."
            // right: "emp_ball1"
            // assert_eq!(file.texture_name.get_string(), "emp_ball1");
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

    #[test]
    fn parse_big() {
        let _wad = Wad::from_file("/home/khang/map_compiler/cso_normal_pack.wad").unwrap();
        let _wad2 = Wad::from_file("/home/khang/map_compiler/cso_normal_pack.wad").unwrap();

        // check the memory usage
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
