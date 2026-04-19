use byte_writer::ByteWriter;

use crate::{ModelHeader, Texture};

pub(super) trait WriteToWriter {
    /// Returns the offset to the header of the struct
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();

        // align data for vec type
        writer.align_size(4);

        offset
    }
}

pub(super) trait WriteToWriterTexture {
    /// Returns offset to texture headers and offset to image data
    fn write_to_writer(&self, writer: &mut ByteWriter) -> (usize, usize);
}

pub(super) trait WriteToWriterBodyparts {
    fn write_to_writer(&self, writer: &mut ByteWriter, textures: &[Texture]) -> usize;
}

pub(super) trait WriteToWriterModel {
    fn write_to_writer(&self, writer: &mut ByteWriter, textures: &[Texture]) -> ModelHeader;
}

pub(super) trait WriteToWriterModels {
    fn write_to_writer(&self, writer: &mut ByteWriter, textures: &[Texture]) -> usize;
}
