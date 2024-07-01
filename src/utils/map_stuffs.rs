use glam::{DVec2, Vec4Swizzles};
use map::Brush;
use smd::{Triangle, Vertex};

use crate::utils::simple_calculs::Solid3D;

use super::simple_calculs::{ConvexPolytope, Plane3D, Triangle3D};

use eyre::eyre;

// https://3707026871-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2F-LtVT8pJjInrrHVCovzy%2Fuploads%2FEukkFYJLwfafFXUMpsI2%2FMAPFiles_2001_StefanHajnoczi.pdf?alt=media&token=51471685-bf69-42ae-a015-a474c0b95165
// https://github.com/pwitvoet/mess/blob/master/MESS/Mapping/Brush.cs#L38
pub fn brush_to_triangulated_smd_3_points(brush: &Brush) -> eyre::Result<Vec<Triangle>> {
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

    let plane_count = solid.face_count();

    let mut polytope = ConvexPolytope::with_face_count(plane_count);
    for i in 0..plane_count {
        for j in (i + 1)..plane_count {
            for k in (j + 1)..plane_count {
                let new_vertex =
                    solid.faces()[i].intersect_with_two_planes(solid.faces()[j], solid.faces()[k]);

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

    let triangulatable = polytope
        .polygons()
        .iter()
        .zip(&brush.planes)
        .map(|(polygon, brush_plane)| {
            // So, normal vector will point down on the texture, aka where you are looking at, I think.
            // So for a vector pointing down for a face. You go the "opposite way". The first face you see would be
            // the face having texture.
            let norm = brush_plane.u.xyz().cross(brush_plane.v.xyz());

            if polygon
                .normal()
                .unwrap()
                .normalize()
                .dot(norm.normalize().into())
                >= 0.
            {
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
    Ok(triangulatable
        .into_iter()
        .zip(&brush.planes)
        .flat_map(|(face_3d, brush_plane)| {
            face_3d
                .into_iter()
                .map(|triangle_3d| {
                    let norm = brush_plane.u.xyz().cross(brush_plane.v.xyz());
                    let uv = DVec2 {
                        x: brush_plane.u_scale,
                        y: brush_plane.v_scale,
                    };
                    let parent = 0;

                    Triangle {
                        material: brush_plane.texture_name.to_owned(),
                        vertices: vec![
                            Vertex {
                                parent,
                                pos: triangle_3d.get_triangle()[0].into(),
                                norm,
                                uv,
                                source: None,
                            },
                            Vertex {
                                parent,
                                pos: triangle_3d.get_triangle()[1].into(),
                                norm,
                                uv,
                                source: None,
                            },
                            Vertex {
                                parent,
                                pos: triangle_3d.get_triangle()[2].into(),
                                norm,
                                uv,
                                source: None,
                            },
                        ],
                    }
                })
                .collect::<Vec<Triangle>>()
        })
        .collect::<Vec<Triangle>>())
}

// subtractive geometry method for crazy speed up
pub fn brush_to_triangulated_smd_subtractive(brush: Brush) -> eyre::Result<Vec<Triangle>> {
    todo!()
}

#[cfg(test)]
mod test {
    use smd::Smd;

    use super::*;

    fn default_cube() -> Brush {
        // cube is 32 x 32 x 32
        // origin is the origin
        let default_cube_str = "\
( -16 -16 16 ) ( -16 16 -16 ) ( -16 16 16 ) devcrate64.bmp [ 0 -1 0 0 ] [ -0 -0 -1 0 ] 0 1 1
( 16 -16 16 ) ( -16 -16 -16 ) ( -16 -16 16 ) devcrate64.bmp [ 1 -0 0 0 ] [ 0 -0 -1 0 ] 0 1 1
( 16 16 -16 ) ( -16 -16 -16 ) ( 16 -16 -16 ) devcrate64.bmp [ -1 0 -0 0 ] [ -0 -1 0 0 ] 0 1 1
( 16 16 16 ) ( -16 -16 16 ) ( -16 16 16 ) devcrate64.bmp [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 16 16 16 ) ( -16 16 -16 ) ( 16 16 -16 ) devcrate64.bmp [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 16 16 16 ) ( 16 -16 -16 ) ( 16 -16 16 ) devcrate64.bmp [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
";

        Brush::try_from(default_cube_str).unwrap()
    }

    #[test]
    fn normal_cube() {
        let cube = default_cube();
        let triangles = brush_to_triangulated_smd_3_points(&cube).unwrap();

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
( -16 -22.62741699796952 -45.254833995939045 ) ( -16 -21.920310216782973 -44.5477272147525 ) ( -16 -23.334523779156065 -44.5477272147525 ) __TB_empty [ 0 -0.7071067811865475 -0.7071067811865477 0 ] [ 0 0.7071067811865476 -0.7071067811865475 0 ] 45 1 1
( -64 0 -22.62741699796952 ) ( -64 -0.7071067811865461 -21.920310216782973 ) ( -63 0 -22.62741699796952 ) __TB_empty [ 1 0 0 0 ] [ 0 0.7071067811865476 -0.7071067811865475 0 ] 0 1 1
( 64 45.25483399593904 67.88225099390857 ) ( 64 45.961940777125584 68.58935777509511 ) ( 65 45.25483399593904 67.88225099390857 ) the_end_stuck [ 1 0 0 80 ] [ 0 -0.7071067811865475 -0.7071067811865477 16 ] 0 1 1
( -64 -22.62741699796952 -45.254833995939045 ) ( -63 -22.62741699796952 -45.254833995939045 ) ( -64 -21.920310216782973 -44.5477272147525 ) jeniceq [ -1 0 0 112 ] [ 0 -0.7071067811865475 -0.7071067811865477 160 ] 0 1 1
( 64 0 22.62741699796952 ) ( 65 0 22.62741699796952 ) ( 64 -0.7071067811865497 23.33452377915607 ) __TB_empty [ -1 0 0 0 ] [ 0 0.7071067811865476 -0.7071067811865475 0 ] 0 1 1
( 16 45.25483399593904 67.88225099390857 ) ( 16 44.547727214752484 68.58935777509511 ) ( 16 45.961940777125584 68.58935777509511 ) __TB_empty [ 0 0.7071067811865475 0.7071067811865477 0 ] [ 0 0.7071067811865476 -0.7071067811865475 0 ] 315 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd_3_points(&cube).unwrap();

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
( 22.627416997969526 -45.25483399593904 -16 ) ( 21.92031021678298 -44.54772721475249 -16 ) ( 22.627416997969526 -45.25483399593904 -15 ) __TB_empty [ 0.7071067811865476 -0.7071067811865475 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 33.94112549695428 56.568542494923804 16 ) ( 34.648232278140824 57.27564927611035 16 ) ( 33.94112549695428 56.568542494923804 17 ) __TB_empty [ -0.7071067811865475 -0.7071067811865477 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -11.313708498984752 -79.19595949289332 -16 ) ( -10.606601717798206 -78.48885271170678 -16 ) ( -12.020815280171298 -78.48885271170678 -16 ) jeniceq [ -0.7071067811865475 -0.7071067811865477 0 112 ] [ 0.7071067811865476 -0.7071067811865475 0 160 ] 45 1 1
( -11.313708498984766 101.82337649086284 16 ) ( -12.020815280171313 102.5304832720494 16 ) ( -10.60660171779822 102.5304832720494 16 ) the_end_stuck [ 0.7071067811865475 0.7071067811865477 0 80 ] [ 0.7071067811865476 -0.7071067811865475 0 16 ] 315 1 1
( -33.94112549695428 -56.568542494923804 -16 ) ( -33.94112549695428 -56.568542494923804 -15 ) ( -33.23401871576773 -55.86143571373726 -16 ) __TB_empty [ 0.7071067811865475 0.7071067811865477 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -45.254833995939045 67.88225099390856 16 ) ( -45.254833995939045 67.88225099390856 17 ) ( -45.96194077712559 68.58935777509511 16 ) __TB_empty [ -0.7071067811865476 0.7071067811865475 0 0 ] [ 0 0 -1 0 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd_3_points(&cube).unwrap();

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
( -56.5685424949238 -48 33.941125496954285 ) ( -55.86143571373725 -48 33.23401871576774 ) ( -56.5685424949238 -47 33.941125496954285 ) NULL [ -0.7071067811865475 0 0.7071067811865477 112 ] [ 0 -1 0 160 ] 0 1 1
( -45.25483399593904 -48 22.627416997969526 ) ( -45.25483399593904 -47 22.627416997969526 ) ( -44.54772721475249 -48 23.334523779156072 ) NULL [ 0 -1 0 0 ] [ -0.7071067811865476 0 -0.7071067811865475 0 ] 0 1 1
( -56.5685424949238 -16 33.941125496954285 ) ( -55.86143571373725 -16 34.64823227814083 ) ( -55.86143571373725 -16 33.23401871576774 ) NULL [ 0.7071067811865475 0 -0.7071067811865477 0 ] [ -0.7071067811865476 0 -0.7071067811865475 0 ] 45 1 1
( 56.5685424949238 16 -33.941125496954285 ) ( 57.27564927611034 16 -34.64823227814083 ) ( 57.27564927611034 16 -33.23401871576774 ) NULL [ -0.7071067811865475 0 0.7071067811865477 0 ] [ -0.7071067811865476 0 -0.7071067811865475 0 ] 315 1 1
( 45.25483399593904 80 -22.627416997969526 ) ( 45.96194077712559 80 -21.92031021678298 ) ( 45.25483399593904 81 -22.627416997969526 ) NULL [ 0 1 0 0 ] [ -0.7071067811865476 0 -0.7071067811865475 0 ] 0 1 1
( 56.5685424949238 80 -33.941125496954285 ) ( 56.5685424949238 81 -33.941125496954285 ) ( 57.27564927611034 80 -34.64823227814083 ) NULL [ 0.7071067811865475 0 -0.7071067811865477 80 ] [ 0 -1 0 16 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd_3_points(&cube).unwrap();

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
( -16 16 -16 ) ( 0 0 16 ) ( -16 -16 -16 ) NULL [ 2.220446049250313e-16 0 -1 80 ] [ 0 -1 0 16 ] 0 1 1
( 0 0 16 ) ( 16 -16 -16 ) ( -16 -16 -16 ) NULL [ 1 0 0 80 ] [ 0 -2.220446049250313e-16 1 16 ] 0 1 1
( 16 -16 -16 ) ( 16 16 -16 ) ( -16 16 -16 ) NULL [ -1 0 0 112 ] [ 0 -1 0 160 ] 0 1 1
( -16 16 -16 ) ( 16 16 -16 ) ( 0 0 16 ) NULL [ 1 0 0 80 ] [ 0 -2.220446049250313e-16 -1 -16 ] 0 1 1
( 0 0 16 ) ( 16 16 -16 ) ( 16 -16 -16 ) NULL [ 2.220446049250313e-16 0 1 112 ] [ 0 -1 0 16 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd_3_points(&cube).unwrap();

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
( -16 16 16 ) ( -16 -16 16 ) ( -16 -16 -16 ) NULL [ 0 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -16 16 16 ) ( 0 0 32 ) ( -16 -16 16 ) NULL [ 1 0 0 16 ] [ 0 -1 0 16 ] 0 1 1
( -16 -16 16 ) ( 16 -16 16 ) ( 16 -16 -16 ) NULL [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 0 0 32 ) ( 16 -16 16 ) ( -16 -16 16 ) NULL [ 1 0 0 16 ] [ 0 -1 0 16 ] 0 1 1
( 16 -16 -16 ) ( 16 16 -16 ) ( -16 16 -16 ) NULL [ -1 0 0 112 ] [ 0 -1 0 160 ] 0 1 1
( -16 16 16 ) ( 16 16 16 ) ( 0 0 32 ) NULL [ 1 0 0 80 ] [ 0 -1 0 16 ] 0 1 1
( 16 16 -16 ) ( 16 16 16 ) ( -16 16 16 ) NULL [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 0 0 32 ) ( 16 16 16 ) ( 16 -16 16 ) NULL [ 1 0 0 16 ] [ 0 -1 0 16 ] 0 1 1
( 16 -16 16 ) ( 16 16 16 ) ( 16 16 -16 ) NULL [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd_3_points(&cube).unwrap();

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
( -16 -16 -16 ) ( 16 16 -16 ) ( 16 -16 16 ) devcrate64.bmp [ 0.7071067811865476 -0 0.7071067811865476 0 ] [ -0.4082482904638631 -0.8164965809277261 0.4082482904638631 0 ] 0 1 1
( 16 -16 -16 ) ( -16 -16 -16 ) ( 16 -16 16 ) devcrate64.bmp [ 1 0 -0 0 ] [ 0 -0 -1 0 ] 0 1 1
( 16 -16 -16 ) ( 16 16 -16 ) ( -16 -16 -16 ) devcrate64.bmp [ -1 0 0 0 ] [ -0 -1 -0 0 ] 0 1 1
( 16 -16 16 ) ( 16 16 -16 ) ( 16 -16 -16 ) devcrate64.bmp [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
";
        let cube = Brush::try_from(slanted_block).unwrap();
        let triangles = brush_to_triangulated_smd_3_points(&cube).unwrap();

        assert_eq!(triangles.len(), 4);

        let mut new_smd = Smd::new_basic();
        triangles.into_iter().for_each(|tri| {
            new_smd.add_triangle(tri);
        });

        new_smd
            .write("/home/khang/gchimp/examples/map2prop/tetrahedron.smd")
            .unwrap();
    }
}
