use byte_writer::ByteWriter;

use crate::{writer::WriteToWriter, Hitbox};

impl WriteToWriter for Hitbox {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let Hitbox {
            bone,
            group,
            bbmin,
            bbmax,
        } = self;

        let offset = writer.get_offset();

        writer.append_i32(*bone);
        writer.append_i32(*group);
        writer.append_f32_slice(bbmin.to_array().as_slice());
        writer.append_f32_slice(bbmax.to_array().as_slice());

        offset
    }
}

impl WriteToWriter for &[Hitbox] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();

        self.iter().for_each(|hitbox| {
            hitbox.write_to_writer(writer);
        });

        offset
    }
}
