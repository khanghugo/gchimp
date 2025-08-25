use byte_writer::ByteWriter;

use crate::{writer::WriteToWriter, Sequence};

mod blend;
mod sequence_group;

impl WriteToWriter for &[Sequence] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        todo!()
    }
}
