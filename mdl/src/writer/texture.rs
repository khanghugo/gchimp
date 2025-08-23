use byte_writer::ByteWriter;

use crate::{
    writer::{WriteToWriter, PADDING_MAGIC},
    Bone, BoneController, Mdl,
};

impl Mdl {
    // must write all the texture headers then write data
    pub(super) fn write_textures(&self, writer: &mut ByteWriter) -> usize {
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

        let texture_data_start = writer.get_offset();

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

        texture_data_start
    }
}
