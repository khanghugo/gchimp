use eyre::eyre;
use map::Map;

#[allow(unused)]
pub fn make_func_door_every_xth_block(map: &Map) -> eyre::Result<Map> {
    // modify here
    let entity_idx = 1;
    let stride = 32;

    let mut res = map.clone();
    let func_group = res.entities.remove(entity_idx);

    if func_group.brushes.is_none() {
        return Err(eyre!("Selected entity has no brushes"));
    }

    // lip does not work
    // angle 90 0 0
    // spawnflags 256 so that the map works in LAN
    let func_door_attributes_str = r#""classname" "func_door"
"delay" "0"
"speed" "180"
"movesnd" "0"
"stopsnd" "0"
"wait" "1"
"lip" "0"
"dmg" "0"
"health" "0"
"locked_sound" "0"
"unlocked_sound" "0"
"locked_sentence" "0"
"unlocked_sentence" "0"
"rendermode" "0"
"renderamt" "255"
"rendercolor" "0 0 0"
"renderfx" "0"
"angles" "90 0 0"
"zhlt_lightflags" "0"
"zhlt_noclip" "0"
"style" "0"
"spawnflags" "256"g"#;
    let (_parse_remain, func_door_attribute) =
        map::parser::parse_attributes(func_door_attributes_str).unwrap();

    assert!(_parse_remain.is_empty());

    let mut func_doors = (0..stride)
        .map(|offset| {
            func_group
                .brushes
                .as_ref()
                .unwrap()
                .iter()
                .skip(offset)
                .step_by(stride)
                .cloned()
                .collect::<Vec<map::Brush>>()
        })
        .map(|brushes| map::Entity {
            attributes: func_door_attribute.clone(),
            brushes: brushes.into(),
        })
        .collect::<Vec<map::Entity>>();

    res.entities.append(&mut func_doors);

    Ok(res)
}

#[cfg(test)]
mod test {
    use map::Map;

    use crate::modules::___random_specific_stuffs::make_func_door_every_xth_block::make_func_door_every_xth_block;

    #[test]
    fn run() {
        let mm_path = "/home/khang/map/arte_farte/arte_farte.map";
        let mm = Map::from_file(mm_path).unwrap();
        let mm = make_func_door_every_xth_block(&mm).unwrap();

        mm.write(mm_path).unwrap();
    }
}
