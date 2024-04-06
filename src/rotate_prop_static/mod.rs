use rayon::prelude::*;

use map::Map;

/// Rotate the Z in (Y Z X) for every prop_static by +90
///
/// This will fix Source map import.
///
/// Mutate the input map data.
pub fn rotate_prop_static(map: &mut Map, rename: Option<&str>) {
    map.entities.par_iter_mut().for_each(|entity| {
        let exist = entity
            .attributes
            .get("classname")
            .map(|value| value == "prop_static");
        if let Some(exist) = exist {
            if exist {
                let values = entity.attributes.get("angles").map(|angles| {
                    angles
                        .split_ascii_whitespace()
                        .map(|v| v.parse::<f64>().unwrap())
                });

                if let Some(values) = values {
                    let values: Vec<f64> = values.collect();
                    entity.attributes.insert(
                        "angles".to_string(),
                        format!("{} {} {}", values[0], values[1] + 90., values[2]),
                    );

                    if let Some(rename) = rename {
                        entity
                            .attributes
                            .insert("classname".to_string(), rename.to_string());
                    }
                }
            }
        }
    });
}
