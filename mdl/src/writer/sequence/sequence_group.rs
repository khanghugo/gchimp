use byte_writer::ByteWriter;

use crate::{writer::WriteToWriter, SequenceGroup};

impl WriteToWriter for SequenceGroup {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let SequenceGroup {
            label,
            name,
            unused1,
            unused2,
        } = self;

        let offset = writer.get_offset();

        writer.append_u8_slice(label.as_slice());
        writer.append_u8_slice(name.as_slice());
        writer.append_i32(*unused1);
        writer.append_i32(*unused2);

        offset
    }
}

impl WriteToWriter for &[SequenceGroup] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        self.iter()
            .map(|sequence_group| sequence_group.write_to_writer(writer))
            .collect::<Vec<usize>>()
            .first()
            .cloned()
            .unwrap_or(0)
    }
}
