use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use byte_writer::ByteWriter;
use glam::Vec3;

use crate::{
    error::MdlError,
    writer::impl_trait::{WriteToWriter, WriteToWriterTexture},
    Header, Mdl,
};

mod attachment;
mod bodypart;
mod bone;
mod hitbox;
mod impl_trait;
mod others;
mod sequence;
mod texture;

const MAGIC: &str = "IDST";
const PADDING_MAGIC: i32 = 0x69696969;

impl Mdl {
    pub fn write_to_file(&self, path: impl AsRef<Path> + Into<PathBuf>) -> Result<(), MdlError> {
        let bytes = self.write_to_bytes();

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        file.write_all(&bytes)?;

        file.flush()?;

        Ok(())
    }

    pub fn write_to_bytes(&self) -> Vec<u8> {
        let mut writer = ByteWriter::new();

        //
        // header
        //

        let header_start = writer.get_offset();

        let header = &self.header;
        writer.append_string(MAGIC);
        writer.append_i32(header.version);
        writer.append_u8_slice(header.name.as_slice());

        let file_length_pos = writer.get_offset();
        writer.append_i32(PADDING_MAGIC);

        // nice reasoning
        let mut write_vec3 = |i: Vec3| {
            writer.append_f32_slice(i.to_array().as_slice());
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

        let header_end = writer.get_offset();

        assert_eq!(header_end - header_start, std::mem::size_of::<Header>());

        //
        // write data now
        //

        let bone_offset = self.bones.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(bone_index, bone_offset as i32);

        println!("bone file length {}", writer.get_offset());

        let bone_controller_offset = self
            .bone_controllers
            .as_slice()
            .write_to_writer(&mut writer);
        writer.replace_with_i32(bone_controller_index, bone_controller_offset as i32);

        println!("bone controller file length {}", writer.get_offset());

        let hitbox_offset = self.hitboxes.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(hitbox_index, hitbox_offset as i32);

        println!("hitbox file length {}", writer.get_offset());

        let sequence_offset = self.sequences.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(sequence_index, sequence_offset as i32);

        println!("sequence file length {}", writer.get_offset());

        let sequence_group_offset = self.sequence_groups.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(sequence_group_index, sequence_group_offset as i32);

        println!("sequence group file length {}", writer.get_offset());

        let (texture_offset, texture_image_offset) =
            self.textures.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(texture_index, texture_offset as i32);
        writer.replace_with_i32(texture_data_index, texture_image_offset as i32);

        println!("texture file length {}", writer.get_offset());

        let skin_offset = self.skin_families.write_to_writer(&mut writer);
        writer.replace_with_i32(skin_index, skin_offset as i32);

        println!("skin length {}", writer.get_offset());

        let bodypart_offset = self.bodyparts.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(bodypart_index, bodypart_offset as i32);

        println!("bodypart length {}", writer.get_offset());

        let attachment_offset = self.attachments.as_slice().write_to_writer(&mut writer);
        writer.replace_with_i32(attachment_index, attachment_offset as i32);

        println!("attachment length {}", writer.get_offset());

        let transition_offset = self.transitions.write_to_writer(&mut writer);
        writer.replace_with_i32(transitions_index, transition_offset as i32);

        println!("transitions length {}", writer.get_offset());

        writer.replace_with_i32(file_length_pos, writer.get_offset() as i32);

        writer.data
    }
}
