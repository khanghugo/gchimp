use byte_writer::ByteWriter;

use crate::{writer::WriteToWriter, Bone, BoneController, Mdl};

impl Mdl {
    pub(super) fn write_bones(&self, writer: &mut ByteWriter) {
        self.bones.iter().for_each(|bone| {
            bone.write_to_writer(writer);
        });
    }

    pub(super) fn write_bone_controllers(&self, writer: &mut ByteWriter) {
        self.bone_controllers.iter().for_each(|bone_controller| {
            bone_controller.write_to_writer(writer);
        });
    }
}

impl WriteToWriter for Bone {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let Bone {
            name,
            parent,
            flags,
            bone_controller,
            value,
            scale,
        } = self;

        let offset = writer.get_offset();

        writer.append_u8_slice(name.as_slice());
        writer.append_i32(*parent);
        writer.append_i32(*flags);
        writer.append_i32_slice(bone_controller.as_slice());
        writer.append_f32_slice(value.as_slice());
        writer.append_f32_slice(scale.as_slice());

        offset
    }
}

impl WriteToWriter for BoneController {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> usize {
        let BoneController {
            bone,
            type_,
            start,
            end,
            rest,
            index,
        } = self;
        let offset = writer.get_offset();

        writer.append_i32(*bone);
        writer.append_i32(*type_);
        writer.append_f32(*start);
        writer.append_f32(*end);
        writer.append_i32(*rest);
        writer.append_i32(*index);

        offset
    }
}
