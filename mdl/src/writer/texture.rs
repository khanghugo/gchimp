use byte_writer::ByteWriter;

use crate::{writer::impl_trait::WriteToWriterTexture, Texture, TextureHeader};

impl WriteToWriterTexture for &[Texture] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> (usize, usize) {
        // write all textures then write header
        let texture_image_offset = writer.get_offset();

        let image_offsets: Vec<usize> = self
            .iter()
            .map(|texture| {
                let offset = writer.get_offset();

                let Texture {
                    header,
                    image,
                    palette,
                } = texture;

                assert_eq!(image.len(), (header.width * header.height) as usize);

                writer.append_u8_slice(&image);
                writer.append_u8_slice(
                    palette
                        .iter()
                        .flatten()
                        .cloned()
                        .collect::<Vec<u8>>()
                        .as_slice(),
                );

                offset
            })
            .collect();

        // write header with known image data offsets
        let texture_header_offset = writer.get_offset();

        self.iter()
            .zip(image_offsets)
            .for_each(|(texture, image_offset)| {
                let TextureHeader {
                    name,
                    flags,
                    width,
                    height,
                    index: _,
                } = &texture.header;

                writer.append_u8_slice(name.as_slice());
                writer.append_i32(flags.bits());
                writer.append_i32(*width);
                writer.append_i32(*height);
                writer.append_i32(image_offset as i32);
            });

        (texture_header_offset, texture_image_offset)
    }
}
