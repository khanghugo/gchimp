pub static MAX_GOLDSRC_TEXTURE_SIZE: u32 = 512;

// divided by 2 just to be safe
// divided by 2 again because what the fuck
pub static MAX_SMD_TRIANGLE: usize = 4080 / 2 / 2;

pub static STUDIOMDL_ERROR_PATTERN: &str = "************ ERROR ************";
pub static MAX_GOLDSRC_MODEL_TEXTURE_COUNT: usize = 64;

pub static PALETTE_PAD_COLOR: [u8; 3] = [0, 0, 0];
pub static PALETTE_TRANSPARENT_COLOR: [u8; 3] = [0, 255, 0];

pub static ORIGIN_TEXTURE: &str = "ORIGIN";
pub static NO_RENDER_TEXTURE: &[&str] = &[
    "NULL",
    "HINT",
    "AAATRIGGER",
    "SKIP",
    "sky",
    ORIGIN_TEXTURE,
    "CLIP",
];

pub static EPSILON: f64 = 0.000001;
