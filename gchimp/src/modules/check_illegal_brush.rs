use map::Map;

pub fn check_illegal_brush(map: &Map) {
    map.entities
        .iter()
        .enumerate()
        .for_each(|(entity_idx, entity)| {
            if let Some(brushes) = &entity.brushes {
                brushes.iter().enumerate().for_each(|(brush_idx, brush)| {
                    if brush.planes.len() >= 32 {
                        println!(
                            "Entity {entity_idx} Brush {brush_idx} ( {} {} {} ) ( {} {} {} ) ( {} {} {} ) might be illegal: {} faces",
                            brush.planes[0].p1.x, brush.planes[0].p1.y, brush.planes[0].p1.z,
                            brush.planes[0].p2.x, brush.planes[0].p2.y, brush.planes[0].p2.z,
                            brush.planes[0].p3.x, brush.planes[0].p3.y, brush.planes[0].p3.z,
                            brush.planes.len()
                        );
                    }
                });
            }
        });
}
