use crate::{writer::WriteToWriter, AnimValues, Blend};

// Pretty fucking complicated, I guess.
// Should just take a look at HLAM code but I will eventually
// The first 6 u16 are "offsets" where it says where the motion type starts
// 6 * bone count
// We should know the bone count from Blend data type
// It is weird how the data is stored and how the data is read differ.
impl WriteToWriter for Blend {
    fn write_to_writer(&self, writer: &mut byte_writer::ByteWriter) -> usize {
        // need to write the RLE first
        // the RLE contains all frame animation values for motion type for a bone
        todo!()
    }
}

impl WriteToWriter for AnimValues {
    fn write_to_writer(&self, writer: &mut byte_writer::ByteWriter) -> usize {
        let offset = writer.get_offset();
        let frame_count = self.0.len();

        // TODO actually compressing stuffs
        // right now, it just sends all the frames
        writer.append_u8_slice(&[frame_count as u8; 2]);
        writer.append_i16_slice(&self.0);

        // no need to write 0 0 at the end to stop
        // because the run length already meets the frame count
        // writer.append_u8_slice(&[0u8; 2]);

        offset
    }
}
