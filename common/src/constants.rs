use lazy_static::lazy_static;

pub const MAX_GOLDSRC_TEXTURE_SIZE: u32 = 512;

// // divided by 2 just to be safe
// // divided by 2 again because what the fuck
// // ok fuck this, use variable MAX_SMD_TRIANGLE so that it can be configurable
// lazy_static! {
//     pub static ref MAX_SMD_TRIANGLE: usize = std::env::var("MAX_SMD_TRIANGLE")
//         .map(|v| v.parse::<usize>().unwrap_or(1500))
//         .unwrap_or(1500);
// }

pub const MAX_SMD_PER_MODEL: usize = 32;
pub const MAX_SMD_VERTEX: usize = 2048;

pub const STUDIOMDL_ERROR_PATTERN: &str = "************ ERROR ************";
pub const MAX_GOLDSRC_MODEL_TEXTURE_COUNT: usize = 64;

pub const ORIGIN_TEXTURE: &str = "ORIGIN";
pub const CLIP_TEXTURE: &str = "CLIP";
pub const CONTENTWATER_TEXTURE: &str = "CONTENTWATER";

pub const NO_RENDER_TEXTURE: &[&str] = &[
    "NULL",
    "HINT",
    "AAATRIGGER",
    "SKIP",
    "sky",
    ORIGIN_TEXTURE,
    CLIP_TEXTURE,
    CONTENTWATER_TEXTURE,
];
pub const TRENCHBROOM_EMPTY_TEXTURE: &str = "__TB_empty";

pub const TEXTURE_PREFIXES: &[&str] = &["{", "!", "+", "-", "~"];

pub const EPSILON: f64 = 0.0000001;

pub const MODEL_ENTITIES: &[&str] = &["cycler", "cycler_sprite", "env_sprite"];
pub const SOUND_ENTITIES: &[&str] = &[
    "ambient_generic",
    // it is pretty stupid that no one can use it
    // "trigger_cdaudio"
];

pub const SPRITE_ENTITIES: &[&str] = &["env_sprite", "env_beam", "env_glow", "env_laser"];

pub struct NoRenderTexture;

lazy_static! {
    static ref NO_RENDER_TEXTURE_UPPER: Vec<String> =
        NO_RENDER_TEXTURE.iter().map(|t| t.to_uppercase()).collect();
}

impl NoRenderTexture {
    pub fn contains(&self, s: &str) -> bool {
        NO_RENDER_TEXTURE.contains(&s) || NO_RENDER_TEXTURE_UPPER.iter().any(|t| t.as_str() == s)
    }
}

#[repr(u32)]
#[derive(Default, PartialEq, Clone, Copy)]
pub enum RenderMode {
    #[default]
    Normal = 0,
    PureColor = 1,
    Texture = 2,
    Glow = 3,
    Solid = 4,
    Additive = 5,
}

impl TryFrom<u32> for RenderMode {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Normal),
            1 => Ok(Self::PureColor),
            2 => Ok(Self::PureColor),
            3 => Ok(Self::Glow),
            4 => Ok(Self::Solid),
            5 => Ok(Self::Additive),
            _ => Err(()),
        }
    }
}
