use std::collections::HashMap;

use map::{Map, TextureName};

pub const RENAMETEX_ENTITY_NAME: &str = "gchimp_renametex";

const DEFAULT_ORIGIN_KEY: &str = "origin";
const DEFAULT_ANGLES_KEY: &str = "angles";

// rename textures based on `gchimp_renametex` entity
pub fn rename_texture(map: &mut Map) {
    // get mapping
    let mapping: HashMap<String, String> = map
        .entities
        .iter()
        .filter(|x| {
            x.attributes
                .get("classname".into())
                .is_some_and(|classname| classname == RENAMETEX_ENTITY_NAME)
        })
        .flat_map(|entity| {
            entity
                .attributes
                .iter()
                .filter(|(&ref key, _)| {
                    !(key == DEFAULT_ANGLES_KEY || key == DEFAULT_ORIGIN_KEY || key == "classname")
                })
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
        })
        .collect();

    if mapping.is_empty() {
        return;
    }

    // then map
    map.entities
        .iter_mut()
        .filter_map(|entity| entity.brushes.as_mut())
        .for_each(|brushes| {
            brushes.iter_mut().for_each(|brush| {
                brush.planes.iter_mut().for_each(|plane| {
                    mapping
                        .get(&plane.texture_name.get_string())
                        .map(|value| plane.texture_name = TextureName::new(value.to_owned()));
                })
            })
        });
}
