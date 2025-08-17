use byte_writer::ByteWriter;
use glam::Vec3;

use crate::Mdl;

const MAGIC: &str = "IDST";
const PADDING_MAGIC: i32 = 0x69696969;

impl Mdl {
    pub fn write_to_bytes(&self) -> Vec<u8> {
        let mut writer = ByteWriter::new();

        // header
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

        // writer.append_i32(self.);
        writer.append_i32(self.header.transition_index);

        // writer.append_i32(self.header.tran);

        writer.data
    }

    fn write_texture(&self, writer: &mut ByteWriter) {
        let pixel_offsets = self
            .textures
            .iter()
            .map(|texture| {
                writer.append_u8_slice(texture.header.name.as_slice());
                writer.append_i32(texture.header.flags.bits());
                writer.append_i32(texture.header.width);
                writer.append_i32(texture.header.height);

                let index = writer.get_offset();
                writer.append_i32(PADDING_MAGIC);

                index
            })
            .collect::<Vec<usize>>();

        pixel_offsets
            .iter()
            .zip(self.textures.iter())
            .for_each(|(&index, texture)| {
                let start = writer.get_offset();

                writer.replace_with_i32(index, start as i32);

                writer.append_u8_slice(&texture.image);
                writer.append_u8_slice(
                    texture
                        .palette
                        .iter()
                        .cloned()
                        .flatten()
                        .collect::<Vec<u8>>()
                        .as_slice(),
                );
            });
    }
}
