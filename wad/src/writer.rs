use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use byte_writer::ByteWriter;

use crate::{
    constants::MIPTEX_HEADER_LENGTH,
    types::{DirectoryEntry, FileEntry, MipTex, Wad},
};

impl Wad {
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

impl MipTex {
    pub fn write(&self, writer: &mut ByteWriter) {
        let texture_name_bytes = self.texture_name.get_bytes();

        writer.append_u8_slice(texture_name_bytes);
        writer.append_u8_slice(&vec![0u8; 16 - texture_name_bytes.len()]);

        writer.append_u32(self.width);
        writer.append_u32(self.height);

        // mip_offsets
        if !self.is_external() {
            // normal case when we have embedded texture
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
        } else {
            // in this case, just write one offset number 0
            // but due to some compatibilities with our parser, just write it 4 times
            writer.append_u32(0);
            writer.append_u32(0);
            writer.append_u32(0);
            writer.append_u32(0);
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
