use byte_writer::ByteWriter;

use crate::{writer::WriteToWriter, Attachment, Hitbox, Mdl};

impl WriteToWriter for Attachment {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let Attachment {
            name,
            type_,
            bone,
            org,
            vectors,
        } = self;
        let offset = writer.get_offset();

        writer.append_u8_slice(name.as_slice());
        writer.append_i32(*type_);
        writer.append_i32(*bone);
        writer.append_f32_slice(org.to_array().as_slice());

        vectors.iter().for_each(|x| {
            writer.append_f32_slice(x.to_array().as_slice());
        });

        offset
    }
}

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

impl Mdl {
    pub(super) fn write_transitions(&self, writer: &mut ByteWriter) {
        writer.append_u8_slice(&self.transitions);
    }

    pub(super) fn write_hitboxes(&self, writer: &mut ByteWriter) {
        self.hitboxes.iter().for_each(|hitbox| {
            hitbox.write_to_writer(writer);
        });
    }

    pub(super) fn write_skins(&self, writer: &mut ByteWriter) {
        self.skin_families.iter().for_each(|skin_family| {
            skin_family.iter().for_each(|&skin| {
                writer.append_i16(skin);
            });
        });
    }

    pub(super) fn write_attachments(&self, writer: &mut ByteWriter) {
        self.attachments.iter().for_each(|attachment| {
            attachment.write_to_writer(writer);
        });
    }
}
