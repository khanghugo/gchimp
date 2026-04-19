use byte_writer::ByteWriter;

use crate::{Texture, TextureHeader, writer::impl_trait::WriteToWriterTexture};

impl WriteToWriterTexture for &[Texture] {
    fn write_to_writer(&self, writer: &mut ByteWriter) -> (usize, usize) {
        // for compatibility, have to write the header first. stupid? i know
        // but for now, write the actual image texture and pallete in its own vaccum and store offsets
        // because we are smart and we live in 2000 + 26
        let mut image_texture_writer = ByteWriter::new();

        let image_offsets: Vec<usize> = self
            .iter()
            .map(|texture| {
                let offset = image_texture_writer.get_offset();

                let Texture {
                    header,
                    image,
                    palette,
                } = texture;

                assert_eq!(image.len(), (header.width * header.height) as usize);

                image_texture_writer.append_u8_slice(&image);
                image_texture_writer.append_u8_slice(
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

        // header starts first
        let texture_header_offset = writer.get_offset();

        // header length and size is known before hand, this is the constant to get where the image texture is
        let header_finish = size_of::<TextureHeader>() * self.len();
        let header_finish = (header_finish + 3) & !3; // align data :)

        // now, commit headers to the main stream
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
                writer.append_i32((texture_header_offset + header_finish + image_offset) as i32); // actual image data offset starts from file begin
            });

        // align data right away
        writer.align_size(4);

        // then, commit texture data
        let texture_image_offset = writer.get_offset();

        writer.append_u8_slice(&image_texture_writer.data);
        writer.align_size(4);

        (texture_header_offset, texture_image_offset)
    }
}
