use std::collections::HashSet;

use map::Map;
use wad::Wad;

pub fn check_missing_texture(map: &Map, wads: &[Wad]) -> Vec<String> {
    let available_textures = wads.iter().fold(vec![], |mut acc, wad| {
        wad.entries.iter().for_each(|entry| {
            acc.push(entry.directory_entry.texture_name.to_string());
        });
        acc
    });

    let available_textures = HashSet::<String>::from_iter(available_textures);

    let textures_in_map = map
        .entities
        .iter()
        .fold(HashSet::<String>::new(), |mut acc, entity| {
            if let Some(brushes) = &entity.brushes {
                for brush in brushes.iter() {
                    for plane in brush.planes.iter() {
                        acc.insert(plane.texture_name.clone());
                    }
                }
            }

            acc
        });

    textures_in_map
        .into_iter()
        .filter_map(|texture| {
            if !available_textures.contains(&texture) {
                Some(texture)
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
}
