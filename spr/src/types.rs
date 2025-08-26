pub struct SprHeader {
    pub id: i32,
    pub version: i32,
    pub orientation: i32,
    pub texture_format: i32,
    pub bounding_radius: f32,
    pub max_width: i32,
    pub max_height: i32,
    pub frame_num: i32,
    pub beam_length: f32,
    pub sync_type: i32,
    pub palette_count: i16,
}

pub type SprPalette = Vec<[u8; 3]>;

pub struct SprFrameHeader {
    pub group: i32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: i32,
    pub height: i32,
}

pub type SprFrameImage = Vec<u8>;

pub struct SprFrame {
    pub header: SprFrameHeader,
    pub image: SprFrameImage,
}

pub struct Spr {
    pub header: SprHeader,
    pub palette: SprPalette,
    pub frames: Vec<SprFrame>,
}
