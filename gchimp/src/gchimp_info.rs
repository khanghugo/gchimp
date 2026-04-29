use std::path::PathBuf;

use bitflags::bitflags;
use map::{Entity, Map};

pub const GCHIMP_INFO_ENTITY: &str = "gchimp_info";

pub const GCHIMP_INFO_HL_PATH: &str = "hl_path";
pub const GCHIMP_INFO_GAMEDIR: &str = "gamedir";

pub const GCHIMP_SPAWNFLAGS_KEY: &str = "spawnflags";

pub struct GchimpInfo {
    entity: Entity,
}

#[derive(Debug, thiserror::Error)]
pub enum GchimpInfoError {
    #[error("gchimp_info does not exist in the map")]
    NoGchimpInfo,
    #[error("Too many gchimp_info is created in the map. There should be only 1 gchimp_info.")]
    TooManyGchimpInfo,
    #[error("Path to Half-Life does not exist: {path}")]
    PathToHL { path: String },
    #[error("Path to Half-Life is empty")]
    PathToHLEmpty,
    #[error("Game mod does not exist: {gamemod}")]
    GameMod { gamemod: String },
    #[error("Game mod is empty")]
    GameModEmpty,
    #[error("\"spawnflags\" key is not a number")]
    SpawnflagsKeyNaN,
    #[error("\"spawnflags\" key is not in gchimp_info")]
    SpawnflagsKeyNone,
}

fn check_gchimp_entity(entity: &Entity) -> Result<(), GchimpInfoError> {
    // check path
    if let Some(hl_path) = entity.attributes.get(GCHIMP_INFO_HL_PATH) {
        let game_path = PathBuf::from(hl_path);

        if !game_path.exists() {
            return Err(GchimpInfoError::PathToHL {
                path: hl_path.to_owned(),
            });
        }

        if let Some(gamedir) = entity.attributes.get(GCHIMP_INFO_GAMEDIR) {
            let mod_path = game_path.join(gamedir);

            if !mod_path.exists() {
                return Err(GchimpInfoError::GameMod {
                    gamemod: mod_path.display().to_string(),
                });
            }
        } else {
            return Err(GchimpInfoError::GameModEmpty);
        }
    } else {
        return Err(GchimpInfoError::PathToHLEmpty);
    }

    // check options
    if let Some(options) = entity.attributes.get(GCHIMP_SPAWNFLAGS_KEY) {
        if options.parse::<u32>().is_err() {
            return Err(GchimpInfoError::SpawnflagsKeyNaN);
        }
    } else {
        return Err(GchimpInfoError::SpawnflagsKeyNone);
    };

    Ok(())
}

impl GchimpInfo {
    pub fn from_map(map: &Map) -> Result<Self, GchimpInfoError> {
        let gchimp_info_entities = map
            .get_entities_by_classname(GCHIMP_INFO_ENTITY)
            .collect::<Vec<_>>();

        if gchimp_info_entities.is_empty() {
            return Err(GchimpInfoError::NoGchimpInfo);
        }

        if gchimp_info_entities.len() != 1 {
            return Err(GchimpInfoError::TooManyGchimpInfo);
        }

        let gchimp_info = gchimp_info_entities[0];

        check_gchimp_entity(gchimp_info)?;

        Ok(Self {
            entity: gchimp_info.clone(),
        })
    }

    pub fn hl_path(&self) -> &str {
        self.entity.attributes.get(GCHIMP_INFO_HL_PATH).unwrap()
    }

    pub fn gamedir(&self) -> &str {
        self.entity.attributes.get(GCHIMP_INFO_GAMEDIR).unwrap()
    }

    pub fn spawnflags(&self) -> GchimpInfoOption {
        self.entity.spawnflags().unwrap().into()
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy)]
    pub struct GchimpInfoOption: u32 {
        const None = 0;
        /// Converts map file to a model
        const Map2MdlConversion = 1 << 0;
        /// Exports map2mdl entity into normal map entity
        ///
        /// Keep this option enabled if the model is already converted and does not need updating.
        /// By doing that, model will not be re-converted on every compile.
        const Map2MdlExport = 1 << 1;
        /// Enables JoinMDL
        const JoinMDL = 1 << 2;
    }
}

impl From<u32> for GchimpInfoOption {
    fn from(value: u32) -> Self {
        Self::from_bits_retain(value)
    }
}
