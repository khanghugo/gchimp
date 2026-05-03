use byte_writer::ByteWriter;

use crate::{AnimValues, Blend, writer::WriteToWriter};

impl WriteToWriter for &[Blend] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();
        // need to write the RLE first
        // the RLE contains all frame animation values for motion type for a bone

        // need to use a different writer so that we can write offset at the end
        let mut motion_data_writer = ByteWriter::new();

        let num_blends = self.len();
        let num_bones = self.first().map(|b| b.len()).unwrap_or(0);

        // [[[offset; 6 motion types]; bone count]; blend count]
        let motion_idx_offsets = self
            .iter()
            .enumerate()
            .map(|(blend_idx, blend)| {
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
                                    let data_offset = motion.write_to_writer(&mut motion_data_writer);
                                    
                                    // The offset is relative to the start of the specific bone's offset struct.
                                    // We must jump over the remaining offset headers for all blends to reach the data.
                                    let base_offset = (num_blends - blend_idx) * num_bones * 12;
                                    
                                    data_offset + base_offset - (bone_idx * 12)
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

        writer.align_size(4);

        offset
    }
}

impl WriteToWriter for AnimValues {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();

        // TODO actually compressing stuffs
        // right now, it just sends all the frames
        for chunk in self.0.chunks(255) {
            writer.append_u8_slice(&[chunk.len() as u8; 2]);
            writer.append_i16_slice(chunk);
        }

        // no need to write 0 0 at the end to stop
        // because the run length already meets the frame count
        // writer.append_u8_slice(&[0u8; 2]);

        offset
    }
}
