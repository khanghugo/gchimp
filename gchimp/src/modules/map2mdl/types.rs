use bitflags::bitflags;
use std::path::PathBuf;

use common::constants::RenderMode;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Map2MdlEntitySpawnflag: u32 {
        const FlatShade = 1 << 0;
        /// Containing the original brush and celshade
        const WithCelShade = 1 << 1;
        /// Turning the brush into just celshade
        const AsCelShade = 1 << 2;
        /// Reverses all normals in the model. This is mainly for reflection scenes.
        const ReverseNormals = 1 << 3;
    }
}

impl From<u32> for Map2MdlEntitySpawnflag {
    fn from(value: u32) -> Self {
        Map2MdlEntitySpawnflag::from_bits_retain(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Map2MdlEntityCelShadeOption {
    pub color: [u8; 3],
    pub distance: f32,
}

impl Default for Map2MdlEntityCelShadeOption {
    fn default() -> Self {
        Self {
            color: [0u8; 3],
            distance: 4.,
        }
    }
}

pub struct Map2MdlOption {
    /// Relative output for the model starting from <gamemod>
    ///
    /// In case this struct is used for converting the entire map,
    /// this must be the absolute output path for the model.
    pub output: PathBuf,
    pub model_entity: String,
    pub cliptype: Map2MdlEntityCliptype,
    pub target_origin: Option<String>,
    pub spawnflags: Map2MdlEntitySpawnflag,
    pub celshade_options: Map2MdlEntityCelShadeOption,
    pub rendermode: RenderMode,
}

impl Default for Map2MdlOption {
    fn default() -> Self {
        Self {
            output: "models/map2mdl.mdl".into(),
            model_entity: "cycler_sprite".into(),
            cliptype: Map2MdlEntityCliptype::NoClip,
            target_origin: None,
            spawnflags: Map2MdlEntitySpawnflag::empty(),
            celshade_options: Map2MdlEntityCelShadeOption {
                color: [0, 0, 0],
                distance: 4.,
            },
            rendermode: RenderMode::Normal,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Map2MdlError {
    #[error("\"output\" must be set")]
    NoOutput,
    #[error("\"model_entity\" must set")]
    NoModelEntity,
    #[error("\"cliptype\" must be set")]
    NoCliptype,
    #[error("Output model name must contain \".mdl\"")]
    OutputNotMdl,
    #[error("Unknown clip value: `{value}`")]
    UnknownClipValue { value: String },
    #[error("Map is empty")]
    EmptyMap,
    #[error("Map does not have \"wad\" key to find used WAD files")]
    NoWadKey,
    #[error("Error: `{value}`")]
    GenericError { value: String },
}

#[derive(Debug, Clone, Copy)]
pub enum Map2MdlEntityCliptype {
    NoClip,
    SameAsBrush,
    BiggestBox,
}

impl TryFrom<u32> for Map2MdlEntityCliptype {
    type Error = Map2MdlError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NoClip),
            1 => Ok(Self::SameAsBrush),
            2 => Ok(Self::BiggestBox),
            x => Err(Map2MdlError::UnknownClipValue {
                value: x.to_string(),
            }),
        }
    }
}

impl TryFrom<&str> for Map2MdlEntityCliptype {
    type Error = Map2MdlError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let v = value
            .parse::<u32>()
            .map_err(|_| Map2MdlError::UnknownClipValue {
                value: value.to_owned(),
            })?;

        Map2MdlEntityCliptype::try_from(v)
    }
}
