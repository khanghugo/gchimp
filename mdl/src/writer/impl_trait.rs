use byte_writer::ByteWriter;

use crate::SequenceHeader;

pub(super) trait WriteToWriter {
    /// Returns the offset to the header of the struct
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize;
}

pub(super) trait WriteToWriterTexture {
    /// Returns offset to texture headers and offset to image data
    fn write_to_writer(&self, writer: &mut ByteWriter) -> (usize, usize);
}

pub(super) trait WriteToWriterSequence {
    fn write_to_writer(&self, sequence_header: &SequenceHeader, writer: &mut ByteWriter);
}
