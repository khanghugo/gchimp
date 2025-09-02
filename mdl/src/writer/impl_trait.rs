use byte_writer::ByteWriter;

pub(super) trait WriteToWriter {
    /// Returns the offset to the header of the struct
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();

        offset
    }
}

pub(super) trait WriteToWriterTexture {
    /// Returns offset to texture headers and offset to image data
    fn write_to_writer(&self, writer: &mut ByteWriter) -> (usize, usize);
}
