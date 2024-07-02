use map::Map;

use crate::utils::constants::NO_RENDER_TEXTURE;

pub fn find_low_scaling(map: &Map) {
    map.entities
        .iter()
        .enumerate()
        .for_each(|(entity_idx, entity)| {
            if let Some(brushes) = &entity.brushes {
                brushes.iter().enumerate().for_each(|(brush_idx, brush)| {
                    let mut is_low = false;
                    let mut low_u = 0.;
                    let mut low_v = 0.;
                    let mut texture_name = "";
                    brush.planes.iter().for_each(|plane| if !NO_RENDER_TEXTURE.contains(&plane.texture_name.as_str()) {
                        // hardcoded to care about default layer only
                        if entity.attributes.get("_tb_layer").is_some() || entity.attributes.get("_tb_id").is_some() {
                            return;
                        }

                        if plane.u_scale < 1. || plane.v_scale < 1. {
                            low_u = plane.u_scale;
                            low_v = plane.v_scale;
                            is_low = true;
                            texture_name = &plane.texture_name;
                        }
                    });

                    if is_low {
                        println!("Entity {} Brush {} ( {} {} {} ) ( {} {} {} ) ( {} {} {} ) has low scaling {} {} {}", entity_idx, brush_idx,
                        brush.planes[0].p1.x, brush.planes[0].p1.y, brush.planes[0].p1.z,
                        brush.planes[0].p2.x, brush.planes[0].p2.y, brush.planes[0].p2.z,
                        brush.planes[0].p3.x, brush.planes[0].p3.y, brush.planes[0].p3.z,
                        low_u, low_v, texture_name
                    );
                    }
                });
            }
        });
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run() {
        let map = Map::from_file("/home/khang/map/surf_ben10/surf_ben10.map").unwrap();

        find_low_scaling(&map);
    }
}
