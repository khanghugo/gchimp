use map::{Map, TextureName};

use crate::utils::simple_calculs::Plane3D;

/// Renaming textures based on its normal
#[allow(unused)]
pub fn normal_rename_tex(map: &Map) -> eyre::Result<Map> {
    let mut res = map.clone();

    res.entities.iter_mut().for_each(|entity| {
        entity.brushes.as_mut().map(|brushes| {
            brushes.iter_mut().for_each(|brush| {
                brush.planes.iter_mut().for_each(|plane| {
                    let plane3d = Plane3D::from_three_points(
                        plane.p1.into(),
                        plane.p2.into(),
                        plane.p3.into(),
                    );

                    let normal = plane3d.normal();

                    if normal.z <= -0.95 {
                        plane.texture_name = TextureName::new("NULL".to_string());
                    }
                });
            })
        });
    });

    Ok(res)
}

#[cfg(test)]
mod test {
    use map::Map;

    use crate::modules::___random_specific_stuffs::normal_rename_tex::normal_rename_tex;

    #[test]
    fn run() {
        let mm_path = "/home/khang/map/arte_farte/arte_farte.map";
        let mm = Map::from_file(mm_path).unwrap();
        let mm = normal_rename_tex(&mm).unwrap();

        mm.write(mm_path).unwrap();
    }
}
