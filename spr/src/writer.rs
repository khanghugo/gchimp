use byte_writer::ByteWriter;

use crate::{Spr, SprFrame, SprFrameHeader, SprFrameImage, SprFrames, SprHeader, SprPalette};

trait WriteToWriter {
    fn write_to_bytes(&self, writer: &mut ByteWriter);
}

impl Spr {
    pub fn write_to_bytes(&self) -> Vec<u8> {
        let mut writer = ByteWriter::new();

        let Self {
            header,
            palette,
            frames,
        } = self;

        header.write_to_bytes(&mut writer);
        palette.write_to_bytes(&mut writer);
        frames.write_to_bytes(&mut writer);

        writer.data
    }
}

impl WriteToWriter for SprHeader {
    fn write_to_bytes(&self, writer: &mut ByteWriter) {
        let Self {
            id,
            version,
            orientation,
            texture_format,
            bounding_radius,
            max_width,
            max_height,
            frame_num,
            beam_length,
            sync_type,
            palette_count,
        } = self;

        writer.append_i32(*id);
        writer.append_i32(*version);
        writer.append_i32(*orientation);
        writer.append_i32(*texture_format);
        writer.append_f32(*bounding_radius);
        writer.append_i32(*max_width);
        writer.append_i32(*max_height);
        writer.append_i32(*frame_num);
        writer.append_f32(*beam_length);
        writer.append_i32(*sync_type);
        writer.append_i16(*palette_count);
    }
}

impl WriteToWriter for SprPalette {
    fn write_to_bytes(&self, writer: &mut ByteWriter) {
        writer.append_u8_slice(self.as_flattened());
    }
}

impl WriteToWriter for SprFrameHeader {
    fn write_to_bytes(&self, writer: &mut ByteWriter) {
        let Self {
            group,
            origin_x,
            origin_y,
            width,
            height,
        } = self;

        writer.append_i32(*group);
        writer.append_i32(*origin_x);
        writer.append_i32(*origin_y);
        writer.append_i32(*width);
        writer.append_i32(*height);
    }
}

impl WriteToWriter for SprFrameImage {
    fn write_to_bytes(&self, writer: &mut ByteWriter) {
        writer.append_u8_slice(self);
    }
}

impl WriteToWriter for SprFrame {
    fn write_to_bytes(&self, writer: &mut ByteWriter) {
        let Self { header, image } = self;

        header.write_to_bytes(writer);
        image.write_to_bytes(writer);
    }
}

impl WriteToWriter for SprFrames {
    fn write_to_bytes(&self, writer: &mut ByteWriter) {
        self.iter().for_each(|frame| frame.write_to_bytes(writer));
    }
}
