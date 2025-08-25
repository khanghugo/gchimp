use byte_writer::ByteWriter;
use glam::Vec3;

use crate::{
    writer::impl_trait::{WriteToWriter, WriteToWriterTexture},
    Mdl, SequenceHeader,
};

mod bodypart;
mod bone;
mod impl_trait;
mod others;
mod sequence;
mod texture;

const MAGIC: &str = "IDST";
const PADDING_MAGIC: i32 = 0x69696969;

impl Mdl {
    pub fn write_to_bytes(&self) -> Vec<u8> {
        let mut writer = ByteWriter::new();

        //
        // header
        //

        let header = &self.header;
        writer.append_string(MAGIC);
        writer.append_i32(header.version);
        writer.append_u8_slice(header.name.as_slice());

        let header_length = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        // nice reasoning
        let mut write_vec3 = |i: Vec3| {
            writer.append_f32(i.x);
            writer.append_f32(i.y);
            writer.append_f32(i.z);
        };

        write_vec3(header.eye_position);
        write_vec3(header.min);
        write_vec3(header.max);
        write_vec3(header.bbmin);
        write_vec3(header.bbmax);

        writer.append_i32(header.flags);

        writer.append_i32(self.bones.len() as i32);
        let bone_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        writer.append_i32(self.bone_controllers.len() as i32);
        let bone_controller_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        writer.append_i32(self.hitboxes.len() as i32);
        let hitbox_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        writer.append_i32(self.sequences.len() as i32);
        let sequence_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        writer.append_i32(self.sequence_groups.len() as i32);
        let sequence_group_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        writer.append_i32(self.textures.len() as i32);
        let texture_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);
        let texture_data_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        writer.append_i32(self.header.num_skin_ref);
        writer.append_i32(self.skin_families.len() as i32);
        let skin_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        writer.append_i32(self.bodyparts.len() as i32);
        let bodypart_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        writer.append_i32(self.attachments.len() as i32);
        let attachment_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        writer.append_i32(self.header.sound_table);
        writer.append_i32(self.header.sound_index);
        writer.append_i32(self.header.sound_groups);
        writer.append_i32(self.header.sound_group_index);

        writer.append_i32(self.transitions.len() as i32);
        let transitions_index = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        //
        // write data now
        //

        writer.replace_with_i32(bone_index, writer.get_offset() as i32);
        self.write_bones(&mut writer);

        writer.replace_with_i32(bone_controller_index, writer.get_offset() as i32);
        self.write_bone_controllers(&mut writer);

        writer.replace_with_i32(hitbox_index, writer.get_offset() as i32);
        self.write_hitboxes(&mut writer);

        let sequence_offset = self.sequences.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(sequence_index, sequence_offset as i32);

        let sequence_group_offset = self.sequence_groups.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(sequence_group_index, sequence_group_offset as i32);

        let (texture_offset, texture_image_offset) =
            self.textures.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(texture_index, texture_offset as i32);
        writer.replace_with_i32(texture_data_index, texture_image_offset as i32);

        writer.replace_with_i32(skin_index, writer.get_offset() as i32);
        self.write_skins(&mut writer);

        let bodypart_offset = self.bodyparts.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(bodypart_index, bodypart_offset as i32);

        writer.replace_with_i32(attachment_index, writer.get_offset() as i32);
        self.write_attachments(&mut writer);

        writer.replace_with_i32(transitions_index, writer.get_offset() as i32);
        self.write_transitions(&mut writer);

        writer.replace_with_i32(header_length, writer.get_offset() as i32);

        writer.data
    }
}

impl WriteToWriter for SequenceHeader {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let offset = writer.get_offset();

        let SequenceHeader {
            label,
            fps,
            flags,
            activity,
            act_weight,
            num_events,
            event_index,
            num_frames,
            num_pivots,
            pivot_index,
            motion_type,
            motion_bone,
            linear_movement,
            auto_move_pos_index,
            auto_move_angle_index,
            bbmin,
            bbmax,
            num_blends,
            anim_index,
            blend_type,
            blend_start,
            blend_end,
            blend_parent,
            seq_group,
            entry_node,
            exit_node,
            node_flags,
            next_seq,
        } = self;

        writer.append_u8_slice(label.as_slice());
        writer.append_f32(*fps);
        writer.append_i32(flags.bits());
        writer.append_i32(*activity);
        writer.append_i32(*act_weight);
        writer.append_i32(*num_events);
        writer.append_i32(*event_index);
        writer.append_i32(*num_frames);
        writer.append_i32(*num_pivots);
        writer.append_i32(*pivot_index);
        writer.append_i32(*motion_type);
        writer.append_i32(*motion_bone);
        writer.append_f32_slice(linear_movement.to_array().as_slice());
        writer.append_i32(*auto_move_pos_index);
        writer.append_i32(*auto_move_angle_index);
        writer.append_f32_slice(bbmin.to_array().as_slice());
        writer.append_f32_slice(bbmax.to_array().as_slice());
        writer.append_i32(*num_blends);
        writer.append_i32(*anim_index);
        writer.append_i32_slice(blend_type.as_slice());
        writer.append_f32_slice(blend_start.as_slice());
        writer.append_f32_slice(blend_end.as_slice());
        writer.append_i32(*blend_parent);
        writer.append_i32(*seq_group);
        writer.append_i32(*entry_node);
        writer.append_i32(*exit_node);
        writer.append_i32(*node_flags);
        writer.append_i32(*next_seq);

        offset
    }
}
