pub static MAX_GOLDSRC_TEXTURE_SIZE: u32 = 512;

// divided by 2 just to be safe
// divided by 2 again because what the fuck
pub static MAX_SMD_TRIANGLE: usize = 1500;
pub static MAX_SMD_PER_MODEL: usize = 32;

pub static STUDIOMDL_ERROR_PATTERN: &str = "************ ERROR ************";
pub static MAX_GOLDSRC_MODEL_TEXTURE_COUNT: usize = 64;

pub static PALETTE_PAD_COLOR: [u8; 3] = [0, 0, 0];
pub static PALETTE_TRANSPARENT_COLOR: [u8; 3] = [0, 255, 0];
pub static PALETTE_TRANSPARENT_COLOR2: [u8; 3] = [0, 0, 255];

pub static ORIGIN_TEXTURE: &str = "ORIGIN";
pub static CLIP_TEXTURE: &str = "CLIP";
pub static CONTENTWATER_TEXTURE: &str = "CONTENTWATER";

pub static NO_RENDER_TEXTURE: &[&str] = &[
    "NULL",
    "HINT",
    "AAATRIGGER",
    "SKIP",
    "sky",
    ORIGIN_TEXTURE,
    CLIP_TEXTURE,
    CONTENTWATER_TEXTURE,
];
pub static TRENCHBROOM_EMPTY_TEXTURE: &str = "__TB_empty";

pub static TEXTURE_PREFIXES: &[&str] = &["{", "!", "+", "-", "~"];

pub static EPSILON: f64 = 0.0000001;
