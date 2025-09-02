use byte_writer::ByteWriter;

use crate::{writer::WriteToWriter, AnimValues, Blend};

impl WriteToWriter for &[Blend] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();
        // need to write the RLE first
        // the RLE contains all frame animation values for motion type for a bone

        // need to use a different writer so that we can write offset at the end
        let mut motion_data_writer = ByteWriter::new();

        // [[[offset; 6 motion types]; bone count]; blend count]
        let motion_idx_offsets = self
            .iter()
            .map(|blend| {
                blend
                    .iter()
                    .enumerate()
                    .map(|(bone_idx, bone)| {
                        bone
                            .iter()
                            .map(|motion|
                                // need to offset 12 here to have the offset start correctly at the "offset position"
                                if motion.is_zero() {
                                    0
                                } else {
                                    motion.write_to_writer(&mut motion_data_writer) - (bone_idx * 12)
                                }
                                )
                            .collect()
                    })
                    .collect()
            })
            .collect::<Vec<Vec<Vec<usize>>>>();

        // now write all the offsets
        motion_idx_offsets.iter().for_each(|blend| {
            blend.iter().for_each(|bone| {
                bone.iter().for_each(|offset| {
                    // using main writer now
                    // make sure this is u16
                    writer.append_u16(*offset as u16);
                });
            });
        });

        // then add the motion data to the writer
        writer.append_u8_slice(&motion_data_writer.data);

        offset
    }
}

impl WriteToWriter for AnimValues {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
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
