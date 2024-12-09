use std::{
    ffi::OsStr,
    fmt::{self, Display, Write as FmtWrite},
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    str::from_utf8,
};

use byte_writer::ByteWriter;
use eyre::eyre;

use crate::{
    constants::{MAX_TEXTURE_NAME_LENGTH, MIPTEX_HEADER_LENGTH},
    parser::parse_wad,
};

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

#[derive(Debug, Clone)]
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

#[derive(Clone)]
/// Don't use to_string() method.
///
/// Use get_string() instead
pub struct TextureName(pub Vec<u8>);

impl fmt::Debug for TextureName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&self.get_string()).field(&self.0).finish()
    }
}

impl TextureName {
    // impl Debug to_string is the same as this
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

    /// Texture name will all be upper case
    pub fn get_string_standard(&self) -> String {
        self.get_string().to_uppercase()
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
            return Err(eyre!("max length for name is 15 characters."));
        }

        if s.as_ref().contains(" ") {
            return Err(eyre!("name should not contain empty space"));
        }

        self.0[..s.as_ref().len()].copy_from_slice(s.as_ref().as_bytes());

        self.0[s.as_ref().len()] = 0;

        Ok(())
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

#[derive(Debug, Clone)]
pub struct Image(pub Vec<u8>);

impl Image {
    pub fn new(s: impl AsRef<[u8]> + Into<Vec<u8>>) -> Self {
        Self(s.into())
    }

    pub fn get_bytes(&self) -> &Vec<u8> {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct Palette(pub Vec<[u8; 3]>);

impl Palette {
    pub fn new(s: impl Into<Vec<[u8; 3]>>) -> Self {
        Self(s.into())
    }

    pub fn get_bytes(&self) -> &Vec<[u8; 3]> {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct Qpic {
    pub width: u32,
    pub height: u32,
    // [[u8; width]; height]
    pub data: Image,
    pub colors_used: i16,
    // Vec<[u8; 3]>
    pub palette: Palette,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

    /// Returns RGB image and dimensions
    pub fn to_rgb(&self) -> (Vec<u8>, (u32, u32)) {
        let image = self.mip_images[0]
            .data
            .get_bytes()
            .iter()
            .flat_map(|&palette_idx| self.palette.get_bytes()[palette_idx as usize])
            .collect::<Vec<u8>>();

        (image, (self.width, self.height))
    }

    /// Returns RGBA image and dimensions
    pub fn to_rgba(&self) -> (Vec<u8>, (u32, u32)) {
        let image = self.mip_images[0]
            .data
            .get_bytes()
            .iter()
            .flat_map(|&palette_idx| {
                let [r, g, b] = self.palette.get_bytes()[palette_idx as usize];
                [r, g, b, 255]
            })
            .collect::<Vec<u8>>();

        (image, (self.width, self.height))
    }

    pub fn write(&self, writer: &mut ByteWriter) {
        let texture_name_bytes = self.texture_name.get_bytes();
        writer.append_u8_slice(texture_name_bytes);
        writer.append_u8_slice(&vec![0u8; 16 - texture_name_bytes.len()]);

        writer.append_u32(self.width);
        writer.append_u32(self.height);

        // mip_offsets
        writer.append_u32(MIPTEX_HEADER_LENGTH);
        writer.append_u32(MIPTEX_HEADER_LENGTH + self.width * self.height);
        writer.append_u32(
            MIPTEX_HEADER_LENGTH + self.width * self.height + (self.width * self.height) / 4,
        );
        writer.append_u32(
            MIPTEX_HEADER_LENGTH
                + self.width * self.height
                + (self.width * self.height) / 4
                + (self.width * self.height) / 4 / 4,
        );

        // if no mipimages then don't write anything more
        if self.mip_images.is_empty() {
            return;
        }

        // mip images
        for image in &self.mip_images {
            writer.append_u8_slice(image.data.get_bytes());
        }

        // colors_used
        writer.append_i16(256);

        for row in self.palette.get_bytes() {
            writer.append_u8_slice(row);
        }

        // pad palette to correctly have 256 colors
        writer.append_u8_slice(&vec![0u8; (256 - self.palette.get_bytes().len()) * 3]);
    }
}

#[derive(Debug, Clone)]
pub struct CharInfo {
    pub offset_y: i8,
    pub offset_x: i8,
    pub charwidth: i16,
}

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

    pub fn texture_name_standard(&self) -> String {
        self.directory_entry.texture_name.get_string_standard()
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

#[derive(Debug, Clone)]
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

        // TODO:write num_dirs with the count of entries
        // doing this will help with forgetting to update num_dirs when new MipMap is added

        // writer.append_i32(self.entries.len() as i32);
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
                let file_entry_offset = writer.get_offset();

                // write file entry
                match file_entry {
                    FileEntry::Qpic(_) => unimplemented!(),
                    FileEntry::MipTex(miptex) => {
                        miptex.write(&mut writer);
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
