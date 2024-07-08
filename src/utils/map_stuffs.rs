use std::collections::HashSet;

use glam::{DVec2, DVec3, Vec4Swizzles};
use map::{Brush, BrushPlane, Entity, Map};
use smd::{Triangle, Vertex};

use crate::utils::simple_calculs::Solid3D;

use super::{
    simple_calculs::{ConvexPolytope, Plane3D, Triangle3D},
    wad_stuffs::SimpleWad,
};

use eyre::eyre;

use rayon::prelude::*;

static SUBTRACTIVE_CUBE_SIZE: f64 = 128000.;

/// Remember to check if texture exists.
pub fn map_to_triangulated_smd(
    map: &Map,
    wads: &SimpleWad,
    three_planes: bool,
) -> eyre::Result<Vec<Triangle>> {
    let res = map
        .entities
        .par_iter()
        .filter(|entity| entity.brushes.is_some()) // for entities with brush only
        .map(|entity| entity_to_triangulated_smd_3_points(entity, wads, three_planes))
        .collect::<Vec<eyre::Result<Vec<Triangle>>>>();

    let err = res
        .iter()
        .filter_map(|res| res.as_ref().err())
        .fold(String::new(), |acc, e| acc + e.to_string().as_ref() + "\n");

    if !err.is_empty() {
        return Err(eyre!("{}", err));
    }

    Ok(res
        .into_iter()
        .filter_map(|res| res.ok())
        .flatten()
        .collect())
}

/// Remember to check if texture exists.
pub fn entity_to_triangulated_smd_3_points(
    entity: &Entity,
    wads: &SimpleWad,
    three_planes: bool,
) -> eyre::Result<Vec<Triangle>> {
    if entity.brushes.is_none() {
        return Err(eyre!("This entity does not contain any brushes."));
    }

    let res = entity
        .brushes
        .as_ref()
        .unwrap()
        .par_iter()
        .map(|brush| brush_to_triangulated_smd(brush, wads, three_planes))
        .collect::<Vec<eyre::Result<Vec<Triangle>>>>();

    let err = res
        .iter()
        .filter_map(|res| res.as_ref().err())
        .fold(String::new(), |acc, e| acc + e.to_string().as_ref() + "\n");

    if !err.is_empty() {
        return Err(eyre!("{}", err));
    }

    Ok(res
        .into_iter()
        .filter_map(|res| res.ok())
        .flatten()
        .collect())
}

fn brush_to_triangulated_smd(
    brush: &Brush,
    wads: &SimpleWad,
    three_planes: bool,
) -> eyre::Result<Vec<Triangle>> {
    let solid: Solid3D = brush
        .planes
        .iter()
        .map(|brush_plane| {
            Plane3D::from_three_points(
                brush_plane.p1.into(),
                brush_plane.p2.into(),
                brush_plane.p3.into(),
            )
        })
        .collect::<Vec<Plane3D>>()
        .into();

    // TODO maybe phase out three_planes
    let polytope = if three_planes {
        // https://3707026871-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2F-LtVT8pJjInrrHVCovzy%2Fuploads%2FEukkFYJLwfafFXUMpsI2%2FMAPFiles_2001_StefanHajnoczi.pdf?alt=media&token=51471685-bf69-42ae-a015-a474c0b95165
        // https://github.com/pwitvoet/mess/blob/master/MESS/Mapping/Brush.cs#L38
        let plane_count = solid.face_count();

        let mut polytope = ConvexPolytope::with_face_count(plane_count);

        for i in 0..plane_count {
            for j in (i + 1)..plane_count {
                for k in (j + 1)..plane_count {
                    let new_vertex = solid.faces()[i]
                        .intersect_with_two_planes_fast(solid.faces()[j], solid.faces()[k]);

                    if new_vertex.is_err() || new_vertex.as_ref().unwrap().is_too_big() {
                        continue;
                    }

                    let new_vertex = new_vertex.unwrap();

                    if solid.contains_point(new_vertex) {
                        polytope.polygons_mut()[i].add_vertex(new_vertex);
                        polytope.polygons_mut()[j].add_vertex(new_vertex);
                        polytope.polygons_mut()[k].add_vertex(new_vertex);
                    }
                }
            }
        }

        polytope
    } else {
        // i am very proud that i came up with this shit myself
        let mut polytope = ConvexPolytope::cube(SUBTRACTIVE_CUBE_SIZE);

        solid.faces().iter().for_each(|plane| {
            polytope.cut(plane);
        });

        polytope
    };

    // it is convex so no worry that the center is outside the brush
    let polytope_centroid = polytope.centroid()?;

    let triangulatable = polytope
        .polygons()
        .iter()
        .map(|polygon| {
            // ~~So, normal vector will point down on the texture, aka where you are looking at, I think.~~
            // ~~So for a vector pointing down for a face. You go the "opposite way". The first face you see would be~~
            // ~~the face having texture.~~
            //
            // Apparently, do not trust that the cross would produce the normal.
            // The information is pretty much there and it might be wrong.
            // The correct way to do this is to derive the direction of the normal ourselves with the center of brush.
            // let norm = brush_plane.u.xyz().cross(brush_plane.v.xyz());
            // let texture_direction = polygon
            //     .normal()
            //     .unwrap()
            //     .normalize()
            //     .dot(norm.normalize().into());
            //
            // Luckily, the normal is only important when we need reverse the triangle or not.
            let direction = polygon.centroid().unwrap() - polytope_centroid;
            let texture_direction = direction.dot(polygon.normal().unwrap().normalize());

            if texture_direction.is_sign_negative() {
                polygon.triangulate(true)
            } else {
                polygon.triangulate(false)
            }
        })
        .collect::<Vec<eyre::Result<Vec<Triangle3D>>>>();

    if let Some(err) = triangulatable
        .iter()
        .filter_map(|face| face.as_ref().err())
        .next()
    {
        return Err(eyre!("Cannot triangulate all polygons: {}", err));
    }

    let triangulatable = triangulatable
        .into_iter()
        .map(|face| face.unwrap())
        .collect::<Vec<Vec<Triangle3D>>>();

    // so we have triangulated triangles for a face
    // zip it with list of brush plane from original map because there's nothing changed in order
    // then convert those 3d brush planes into Smd triangle
    let smd_triangles = triangulatable
        .into_iter()
        .zip(&brush.planes)
        .flat_map(|(face_3d, brush_plane)| {
            face_3d
                .into_iter()
                .map(|triangle_3d| {
                    // ~flip the normal vector because this one is actually pointing outward from the texture.~
                    // ~map normal vector points toward the texture.~
                    // Don't trust the UV coordinate to give the correct normal
                    // let norm = brush_plane.u.xyz().cross(brush_plane.v.xyz()) * -1.;
                    let norm = triangle_3d.normal().to_dvec3();

                    // make sure to check the texture exists before running the function
                    // seems very inefficient to do check here instead
                    let tex_dimensions = wads.get(&brush_plane.texture_name).unwrap().dimensions();

                    let p1: DVec3 = triangle_3d.get_triangle()[0].into();
                    let p2: DVec3 = triangle_3d.get_triangle()[1].into();
                    let p3: DVec3 = triangle_3d.get_triangle()[2].into();

                    let parent = 0;

                    let v1_uv = convert_uv_origin(p1, brush_plane, tex_dimensions);
                    let v2_uv = convert_uv_origin(p2, brush_plane, tex_dimensions);
                    let v3_uv = convert_uv_origin(p3, brush_plane, tex_dimensions);

                    Triangle {
                        material: brush_plane.texture_name.to_owned(),
                        vertices: vec![
                            Vertex {
                                parent,
                                pos: p1,
                                norm,
                                uv: v1_uv,
                                source: None,
                            },
                            Vertex {
                                parent,
                                pos: p2,
                                norm,
                                uv: v2_uv,
                                source: None,
                            },
                            Vertex {
                                parent,
                                pos: p3,
                                norm,
                                uv: v3_uv,
                                source: None,
                            },
                        ],
                    }
                })
                .collect::<Vec<Triangle>>()
        })
        .collect::<Vec<Triangle>>();

    Ok(smd_triangles)
}

fn convert_uv_origin(
    p: DVec3,
    brush_plane: &BrushPlane,
    (tex_width, tex_height): (u32, u32),
) -> DVec2 {
    let res = DVec2::new(p.dot(brush_plane.u.xyz()), p.dot(brush_plane.v.xyz()))
        / DVec2::new(tex_width as f64, tex_height as f64) // "modulo"
        / DVec2::new(brush_plane.u_scale, brush_plane.v_scale) // scale
        + DVec2::new(brush_plane.u.w, brush_plane.v.w) / DVec2::new(tex_width as f64, tex_height as f64) // offset
        ;

    // no need to handle rotation because apparently UV vector helps with that
    // let rotation = brush_plane.rotation.to_radians();
    // let res = res
    //     * DVec2::new(
    //         res.x * rotation.cos() - res.y * rotation.sin(),
    //         res.x * rotation.sin() + res.y * rotation.cos(),
    //     );

    res * DVec2::new(1., -1.) // flip the v coordinate because .map points toward the texture
}

pub fn textures_used_in_map(map: &Map) -> HashSet<String> {
    map.entities
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
        })
}

#[cfg(test)]
mod test {
    use smd::Smd;

    use super::*;

    fn default_cube() -> Brush {
        // cube is 32 x 32 x 32
        // origin is the origin
        let default_cube_str = "\
( -16 -16 16 ) ( -16 16 -16 ) ( -16 16 16 ) devcrate64 [ 0 -1 0 0 ] [ -0 -0 -1 0 ] 0 1 1
( 16 -16 16 ) ( -16 -16 -16 ) ( -16 -16 16 ) devcrate64 [ 1 -0 0 0 ] [ 0 -0 -1 0 ] 0 1 1
( 16 16 -16 ) ( -16 -16 -16 ) ( 16 -16 -16 ) devcrate64 [ -1 0 -0 0 ] [ -0 -1 0 0 ] 0 1 1
( 16 16 16 ) ( -16 -16 16 ) ( -16 16 16 ) devcrate64 [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 16 16 16 ) ( -16 16 -16 ) ( 16 16 -16 ) devcrate64 [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 16 16 16 ) ( 16 -16 -16 ) ( 16 -16 16 ) devcrate64 [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
";

        Brush::try_from(default_cube_str).unwrap()
    }

    fn devtex() -> SimpleWad {
        let mut res = SimpleWad::default();

        res.insert("devcrate64".to_owned(), 0, (64, 64));
        res.insert("devcross".to_owned(), 0, (128, 128));
        res.insert("devwallgray".to_owned(), 0, (128, 128));

        res
    }

    #[test]
    fn normal_cube() {
        let cube = default_cube();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        assert_eq!(triangles.len(), 12);

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/cube.smd")
            .unwrap();
    }

    #[test]
    fn roll_cube() {
        let slanted_block = "\
( -16 -22.62741699796952 -45.254833995939045 ) ( -16 -21.920310216782973 -44.5477272147525 ) ( -16 -23.334523779156065 -44.5477272147525 ) devcrate64 [ 0 -0.7071067811865475 -0.7071067811865477 0 ] [ 0 0.7071067811865476 -0.7071067811865475 0 ] 45 1 1
( -64 0 -22.62741699796952 ) ( -64 -0.7071067811865461 -21.920310216782973 ) ( -63 0 -22.62741699796952 ) devcrate64 [ 1 0 0 0 ] [ 0 0.7071067811865476 -0.7071067811865475 0 ] 0 1 1
( 64 45.25483399593904 67.88225099390857 ) ( 64 45.961940777125584 68.58935777509511 ) ( 65 45.25483399593904 67.88225099390857 ) devcrate64 [ 1 0 0 80 ] [ 0 -0.7071067811865475 -0.7071067811865477 16 ] 0 1 1
( -64 -22.62741699796952 -45.254833995939045 ) ( -63 -22.62741699796952 -45.254833995939045 ) ( -64 -21.920310216782973 -44.5477272147525 ) devcrate64 [ -1 0 0 112 ] [ 0 -0.7071067811865475 -0.7071067811865477 160 ] 0 1 1
( 64 0 22.62741699796952 ) ( 65 0 22.62741699796952 ) ( 64 -0.7071067811865497 23.33452377915607 ) devcrate64 [ -1 0 0 0 ] [ 0 0.7071067811865476 -0.7071067811865475 0 ] 0 1 1
( 16 45.25483399593904 67.88225099390857 ) ( 16 44.547727214752484 68.58935777509511 ) ( 16 45.961940777125584 68.58935777509511 ) devcrate64 [ 0 0.7071067811865475 0.7071067811865477 0 ] [ 0 0.7071067811865476 -0.7071067811865475 0 ] 315 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        assert_eq!(triangles.len(), 12);

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/roll_cube.smd")
            .unwrap();
    }

    #[test]
    fn yaw_cube() {
        let slanted_block = "\
( 22.627416997969526 -45.25483399593904 -16 ) ( 21.92031021678298 -44.54772721475249 -16 ) ( 22.627416997969526 -45.25483399593904 -15 ) devcrate64 [ 0.7071067811865476 -0.7071067811865475 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 33.94112549695428 56.568542494923804 16 ) ( 34.648232278140824 57.27564927611035 16 ) ( 33.94112549695428 56.568542494923804 17 ) devcrate64 [ -0.7071067811865475 -0.7071067811865477 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -11.313708498984752 -79.19595949289332 -16 ) ( -10.606601717798206 -78.48885271170678 -16 ) ( -12.020815280171298 -78.48885271170678 -16 ) devcrate64 [ -0.7071067811865475 -0.7071067811865477 0 112 ] [ 0.7071067811865476 -0.7071067811865475 0 160 ] 45 1 1
( -11.313708498984766 101.82337649086284 16 ) ( -12.020815280171313 102.5304832720494 16 ) ( -10.60660171779822 102.5304832720494 16 ) devcrate64 [ 0.7071067811865475 0.7071067811865477 0 80 ] [ 0.7071067811865476 -0.7071067811865475 0 16 ] 315 1 1
( -33.94112549695428 -56.568542494923804 -16 ) ( -33.94112549695428 -56.568542494923804 -15 ) ( -33.23401871576773 -55.86143571373726 -16 ) devcrate64 [ 0.7071067811865475 0.7071067811865477 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -45.254833995939045 67.88225099390856 16 ) ( -45.254833995939045 67.88225099390856 17 ) ( -45.96194077712559 68.58935777509511 16 ) devcrate64 [ -0.7071067811865476 0.7071067811865475 0 0 ] [ 0 0 -1 0 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        assert_eq!(triangles.len(), 12);

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/yaw_cube.smd")
            .unwrap();
    }

    #[test]
    fn roll_prism() {
        let slanted_block = "\
( -56.5685424949238 -48 33.941125496954285 ) ( -55.86143571373725 -48 33.23401871576774 ) ( -56.5685424949238 -47 33.941125496954285 ) devcrate64 [ -0.7071067811865475 0 0.7071067811865477 112 ] [ 0 -1 0 160 ] 0 1 1
( -45.25483399593904 -48 22.627416997969526 ) ( -45.25483399593904 -47 22.627416997969526 ) ( -44.54772721475249 -48 23.334523779156072 ) devcrate64 [ 0 -1 0 0 ] [ -0.7071067811865476 0 -0.7071067811865475 0 ] 0 1 1
( -56.5685424949238 -16 33.941125496954285 ) ( -55.86143571373725 -16 34.64823227814083 ) ( -55.86143571373725 -16 33.23401871576774 ) devcrate64 [ 0.7071067811865475 0 -0.7071067811865477 0 ] [ -0.7071067811865476 0 -0.7071067811865475 0 ] 45 1 1
( 56.5685424949238 16 -33.941125496954285 ) ( 57.27564927611034 16 -34.64823227814083 ) ( 57.27564927611034 16 -33.23401871576774 ) devcrate64 [ -0.7071067811865475 0 0.7071067811865477 0 ] [ -0.7071067811865476 0 -0.7071067811865475 0 ] 315 1 1
( 45.25483399593904 80 -22.627416997969526 ) ( 45.96194077712559 80 -21.92031021678298 ) ( 45.25483399593904 81 -22.627416997969526 ) devcrate64 [ 0 1 0 0 ] [ -0.7071067811865476 0 -0.7071067811865475 0 ] 0 1 1
( 56.5685424949238 80 -33.941125496954285 ) ( 56.5685424949238 81 -33.941125496954285 ) ( 57.27564927611034 80 -34.64823227814083 ) devcrate64 [ 0.7071067811865475 0 -0.7071067811865477 80 ] [ 0 -1 0 16 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        assert_eq!(triangles.len(), 12);

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/roll_prism.smd")
            .unwrap();
    }

    #[test]
    fn square_pyramid() {
        let slanted_block = "\
( -16 16 -16 ) ( 0 0 16 ) ( -16 -16 -16 ) devcrate64 [ 2.220446049250313e-16 0 -1 80 ] [ 0 -1 0 16 ] 0 1 1
( 0 0 16 ) ( 16 -16 -16 ) ( -16 -16 -16 ) devcrate64 [ 1 0 0 80 ] [ 0 -2.220446049250313e-16 1 16 ] 0 1 1
( 16 -16 -16 ) ( 16 16 -16 ) ( -16 16 -16 ) devcrate64 [ -1 0 0 112 ] [ 0 -1 0 160 ] 0 1 1
( -16 16 -16 ) ( 16 16 -16 ) ( 0 0 16 ) devcrate64 [ 1 0 0 80 ] [ 0 -2.220446049250313e-16 -1 -16 ] 0 1 1
( 0 0 16 ) ( 16 16 -16 ) ( 16 -16 -16 ) devcrate64 [ 2.220446049250313e-16 0 1 112 ] [ 0 -1 0 16 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        assert_eq!(triangles.len(), 4 + 2);

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/square_pyramid.smd")
            .unwrap();
    }

    #[test]
    fn house_shape() {
        let slanted_block = "\
( -16 16 16 ) ( -16 -16 16 ) ( -16 -16 -16 ) devcrate64 [ 0 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -16 16 16 ) ( 0 0 32 ) ( -16 -16 16 ) devcrate64 [ 1 0 0 16 ] [ 0 -1 0 16 ] 0 1 1
( -16 -16 16 ) ( 16 -16 16 ) ( 16 -16 -16 ) devcrate64 [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 0 0 32 ) ( 16 -16 16 ) ( -16 -16 16 ) devcrate64 [ 1 0 0 16 ] [ 0 -1 0 16 ] 0 1 1
( 16 -16 -16 ) ( 16 16 -16 ) ( -16 16 -16 ) devcrate64 [ -1 0 0 112 ] [ 0 -1 0 160 ] 0 1 1
( -16 16 16 ) ( 16 16 16 ) ( 0 0 32 ) devcrate64 [ 1 0 0 80 ] [ 0 -1 0 16 ] 0 1 1
( 16 16 -16 ) ( 16 16 16 ) ( -16 16 16 ) devcrate64 [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 0 0 32 ) ( 16 16 16 ) ( 16 -16 16 ) devcrate64 [ 1 0 0 16 ] [ 0 -1 0 16 ] 0 1 1
( 16 -16 16 ) ( 16 16 16 ) ( 16 16 -16 ) devcrate64 [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        assert_eq!(triangles.len(), 14);

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/house_shape.smd")
            .unwrap();
    }

    #[test]
    fn tetrahedron() {
        let slanted_block = "\
( -16 -16 -16 ) ( 16 16 -16 ) ( 16 -16 16 ) devcrate64 [ 0.7071067811865476 -0 0.7071067811865476 0 ] [ -0.4082482904638631 -0.8164965809277261 0.4082482904638631 0 ] 0 1 1
( 16 -16 -16 ) ( -16 -16 -16 ) ( 16 -16 16 ) devcrate64 [ 1 0 -0 0 ] [ 0 -0 -1 0 ] 0 1 1
( 16 -16 -16 ) ( 16 16 -16 ) ( -16 -16 -16 ) devcrate64 [ -1 0 0 0 ] [ -0 -1 -0 0 ] 0 1 1
( 16 -16 16 ) ( 16 16 -16 ) ( 16 -16 -16 ) devcrate64 [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        assert_eq!(triangles.len(), 4);

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/tetrahedron.smd")
            .unwrap();
    }

    #[test]
    fn block_rotated_texture() {
        let slanted_block = "\
( 0 0 0 ) ( 0 1 0 ) ( 0 0 1 ) devcrate64 [ 0 -1 0 0 ] [ -0 -0 -1 0 ] 0 1 1
( 0 0 0 ) ( 0 0 1 ) ( 1 0 0 ) devcrate64 [ 1 0 -0 0 ] [ 0 -0 -1 0 ] 0 1 1
( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) devcrate64 [ -1 0 0 0 ] [ -0 -1 -0 0 ] 0 1 1
( 128 128 64 ) ( 128 129 64 ) ( 129 128 64 ) devcrate64 [ 0.9659258244035115 -0.25881905213951417 0 0 ] [ -0.25881905213951417 -0.9659258244035115 0 0 ] 15 1 1
( 128 128 32 ) ( 129 128 32 ) ( 128 128 33 ) devcrate64 [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 128 128 32 ) ( 128 128 33 ) ( 128 129 32 ) devcrate64 [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/cube.smd")
            .unwrap();
    }

    #[test]
    // testing the float precision
    fn rotated_block() {
        let slanted_block = "\
( -95.42562584220407 -71.61721185363308 162.89245613824502 ) ( -95.42562584220407 -70.91010507244653 162.18534935705847 ) ( -94.92562584220407 -71.00483941793729 163.5048285739408 ) devcrate64 [ 0 -0.7071067811865474 0.7071067811865477 -37.82338 ] [ -0.5000000000000001 -0.6123724356957947 -0.6123724356957945 -39.818367 ] 320.5322 1 1
( -87.42562584220407 -61.81925288250036 172.69041510937774 ) ( -86.55960043841964 -62.172806273093634 172.33686171878446 ) ( -87.42562584220407 -61.11214610131381 171.98330832819119 ) devcrate64 [ -0.8660254037844387 0.3535533905932739 0.35355339059327373 13.08831 ] [ 0 -0.7071067811865474 0.7071067811865477 -37.82338 ] 336.59818 1 1
( -95.42562584220407 -71.61721185363308 162.89245613824502 ) ( -94.92562584220407 -71.00483941793729 163.5048285739408 ) ( -94.55960043841964 -71.97076524422636 162.53890274765178 ) devcrate64 [ 0.9999999999999998 1.0302873457157524e-08 1.0302873457157524e-08 -13.08831 ] [ 1.4570463349738993e-08 -0.7071067811865474 -0.7071067811865471 -39.818367 ] 0 1 1
( 31.42562584220407 -6.766459915428641 46.72387209269332 ) ( 32.291651245988504 -7.120013306021917 46.37031870210004 ) ( 31.92562584220407 -6.154087479732851 47.33624452838911 ) devcrate64 [ -0.8660254037844387 0.3535533905932739 0.35355339059327373 13.08831 ] [ -0.5000000000000001 -0.6123724356957947 -0.6123724356957945 -39.818367 ] 330 1 1
( 87.42562584220406 61.81925288250034 115.30958489062226 ) ( 87.42562584220406 62.52635966368689 114.60247810943571 ) ( 88.2916512459885 61.46569949190706 114.956031500029 ) devcrate64 [ 0.8660254037844387 -0.3535533905932739 -0.35355339059327373 -13.08831 ] [ 0 -0.7071067811865474 0.7071067811865477 -37.82338 ] 23.401838 1 1
( 31.42562584220407 -6.766459915428641 46.72387209269332 ) ( 31.92562584220407 -6.154087479732851 47.33624452838911 ) ( 31.42562584220407 -6.059353134242087 46.016765311506774 ) devcrate64 [ 0 0.7071067811865474 -0.7071067811865477 37.82338 ] [ -0.5000000000000001 -0.6123724356957947 -0.6123724356957945 -39.818367 ] 39.467796 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/rotated_block.smd")
            .unwrap();
    }

    #[test]
    fn normal_cube_new() {
        let cube = default_cube();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        assert_eq!(triangles.len(), 12);

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/cube.smd")
            .unwrap();
    }

    #[test]
    fn block_rotated_texture_new() {
        let slanted_block = "\
( 0 0 0 ) ( 0 1 0 ) ( 0 0 1 ) devcrate64 [ 0 -1 0 0 ] [ -0 -0 -1 0 ] 0 1 1
( 0 0 0 ) ( 0 0 1 ) ( 1 0 0 ) devcrate64 [ 1 0 -0 0 ] [ 0 -0 -1 0 ] 0 1 1
( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) devcrate64 [ -1 0 0 0 ] [ -0 -1 -0 0 ] 0 1 1
( 128 128 64 ) ( 128 129 64 ) ( 129 128 64 ) devcrate64 [ 0.9659258244035115 -0.25881905213951417 0 0 ] [ -0.25881905213951417 -0.9659258244035115 0 0 ] 15 1 1
( 128 128 32 ) ( 129 128 32 ) ( 128 128 33 ) devcrate64 [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 128 128 32 ) ( 128 128 33 ) ( 128 129 32 ) devcrate64 [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd(&cube, &devtex(), false).unwrap();

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/cube.smd")
            .unwrap();
    }

    #[test]
    fn sphere1() {
        let map = Map::from_file("/home/khang/gchimp/examples/map2prop/sphere.map").unwrap();
        let triangles = map_to_triangulated_smd(&map, &devtex(), false).unwrap();

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/sphere1.smd")
            .unwrap();
    }

    #[test]
    fn sphere2() {
        let map = Map::from_file("/home/khang/gchimp/examples/map2prop/sphere2.map").unwrap();
        let triangles = map_to_triangulated_smd(&map, &devtex(), false).unwrap();

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/sphere2.smd")
            .unwrap();
    }
}
