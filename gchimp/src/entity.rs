use std::path::PathBuf;

use bitflags::bitflags;
use map::{Entity, Map};

use crate::err;

pub static GCHIMP_INFO_ENTITY: &str = "gchimp_info";

pub static GCHIMP_INFO_HL_PATH: &str = "hl_path";
pub static GCHIMP_INFO_GAMEDIR: &str = "gamedir";
pub static GCHIMP_INFO_OPTIONS: &str = "options";

pub struct GchimpInfo {
    entity: Entity,
}

fn check_gchimp_entity(entity: &Entity) -> eyre::Result<()> {
    // check path
    if let Some(hl_path) = entity.attributes.get(GCHIMP_INFO_HL_PATH) {
        let game_path = PathBuf::from(hl_path);

        if !game_path.exists() {
            return err!("gchimp_info: Path to Half-Life does not exist: {}", hl_path);
        }

        if let Some(gamedir) = entity.attributes.get(GCHIMP_INFO_GAMEDIR) {
            let mod_path = game_path.join(gamedir);

            if !mod_path.exists() {
                return err!(
                    "gchimp_info: Path to game mod does not exist: {}",
                    mod_path.to_str().unwrap()
                );
            }
        } else {
            return err!("gchimp_info: No game mod provided");
        }
    } else {
        return err!("gchimp_info: No path to Half-Life provided");
    }

    // check options
    if let Some(options) = entity.attributes.get("options") {
        if let Err(err) = options.parse::<usize>() {
            return err!(
                "gchimp_info: Value for \"options\" is not a number: {}",
                err
            );
        }
    } else {
        return err!("gchimp_info: Cannot find \"options\" key");
    };

    Ok(())
}

impl GchimpInfo {
    pub fn from_map(map: &Map) -> eyre::Result<Self> {
        let entity_index = map.entities.iter().position(|entity| {
            entity
                .attributes
                .get("classname")
                .is_some_and(|classname| classname == GCHIMP_INFO_ENTITY)
        });

        if entity_index.is_none() {
            return err!("gchimp_info: Cannot find {}", GCHIMP_INFO_ENTITY);
        }

        let entity_index = entity_index.unwrap();
        let entity = &map.entities[entity_index];

        check_gchimp_entity(entity)?;

        Ok(Self {
            entity: entity.clone(),
        })
    }

    pub fn hl_path(&self) -> &str {
        self.entity.attributes.get(GCHIMP_INFO_HL_PATH).unwrap()
    }

    pub fn gamedir(&self) -> &str {
        self.entity.attributes.get(GCHIMP_INFO_GAMEDIR).unwrap()
    }

    pub fn options(&self) -> GchimpInfoOption {
        self.entity
            .attributes
            .get(GCHIMP_INFO_OPTIONS)
            .unwrap()
            .parse::<u32>()
            .unwrap()
            .into()
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
        /// Keep this option ticked if the model is already converted and model is not updated.
        /// By doing that, model will not be re-converted on every compile.
        const Map2MdlExport = 1 << 1;
    }
}

impl From<u32> for GchimpInfoOption {
    fn from(value: u32) -> Self {
        Self::from_bits(value).unwrap_or_default()
    }
}
