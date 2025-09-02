use byte_writer::ByteWriter;

use crate::{writer::WriteToWriter, Attachment};

impl WriteToWriter for Attachment {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let Attachment {
            name,
            type_,
            bone,
            org,
            vectors,
        } = self;
        let offset = writer.get_offset();

        writer.append_u8_slice(name.as_slice());
        writer.append_i32(*type_);
        writer.append_i32(*bone);
        writer.append_f32_slice(org.to_array().as_slice());

        vectors.iter().for_each(|x| {
            writer.append_f32_slice(x.to_array().as_slice());
        });

        offset
    }
}

impl WriteToWriter for &[Attachment] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();

        self.iter().for_each(|attachment| {
            attachment.write_to_writer(writer);
        });

        offset
    }
}
