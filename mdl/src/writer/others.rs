use byte_writer::ByteWriter;

use crate::{writer::WriteToWriter, SkinFamilies, Transitions};

impl WriteToWriter for SkinFamilies {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();

        self.iter().flatten().for_each(|x| {
            writer.append_i16(*x);
        });

        offset
    }
}

impl WriteToWriter for Transitions {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();

        writer.append_u8_slice(self);

        offset
    }
}
