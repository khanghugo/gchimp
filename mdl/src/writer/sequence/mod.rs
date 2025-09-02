use byte_writer::ByteWriter;

use crate::{writer::WriteToWriter, Sequence, SequenceHeader};

mod blend;
mod sequence_group;

impl WriteToWriter for &[Sequence] {
    fn write_to_writer(&self, mut writer: &mut ByteWriter) -> usize {
        
        // write sequence data and then sequence headers next
        let anim_indices = self
            .iter()
            .map(|sequence| sequence.anim_blends.as_slice().write_to_writer(&mut writer))
            .collect::<Vec<usize>>();

        // write header
        let offsets = writer.get_offset();

        self.iter()
            .zip(anim_indices)
            .for_each(|(sequence, our_anim_index)| {
                let SequenceHeader {
                    label,
                    fps,
                    flags,
                    activity,
                    act_weight,
                    num_events,
                    event_index,
                    num_frames: _,
                    num_pivots,
                    pivot_index,
                    motion_type,
                    motion_bone,
                    linear_movement,
                    auto_move_pos_index,
                    auto_move_angle_index,
                    bbmin,
                    bbmax,
                    num_blends: _,
                    anim_index: _,
                    blend_type,
                    blend_start,
                    blend_end,
                    blend_parent,
                    seq_group,
                    entry_node,
                    exit_node,
                    node_flags,
                    next_seq,
                } = &sequence.header;

                let start = writer.get_offset();

                writer.append_u8_slice(label);
                writer.append_f32(*fps);
                writer.append_i32(flags.bits());
                writer.append_i32(*activity);
                writer.append_i32(*act_weight);
                writer.append_i32(*num_events);
                writer.append_i32(*event_index);
                writer.append_i32(sequence.anim_blends[0][0][0].len() as i32);
                writer.append_i32(*num_pivots);
                writer.append_i32(*pivot_index);
                writer.append_i32(*motion_type);
                writer.append_i32(*motion_bone);
                writer.append_f32_slice(linear_movement.to_array().as_slice());
                writer.append_i32(*auto_move_pos_index);
                writer.append_i32(*auto_move_angle_index);
                writer.append_f32_slice(bbmin.to_array().as_slice());
                writer.append_f32_slice(bbmax.to_array().as_slice());
                writer.append_i32(sequence.anim_blends.len() as i32);
                writer.append_i32(our_anim_index as i32);
                writer.append_i32_slice(blend_type);
                writer.append_f32_slice(blend_start);
                writer.append_f32_slice(blend_end);
                writer.append_i32(*blend_parent);
                writer.append_i32(*seq_group);
                writer.append_i32(*entry_node);
                writer.append_i32(*exit_node);
                writer.append_i32(*node_flags);
                writer.append_i32(*next_seq);

                let end = writer.get_offset();

                assert_eq!(end - start, std::mem::size_of::<SequenceHeader>());
            });

        offsets
    }
}
