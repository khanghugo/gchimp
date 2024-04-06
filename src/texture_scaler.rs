use rayon::prelude::*;

use map::Map;

/// Scale all texture by a scalar.
///
/// Mutate the input map data.
pub fn texture_scaler(map: &mut Map, scalar: f64) {
    map.entities.par_iter_mut().for_each(|entity| {
        if let Some(brushes) = &mut entity.brushes {
            brushes.par_iter_mut().for_each(|brush| {
                brush.planes.par_iter_mut().for_each(|plane| {
                    plane.u_scale *= scalar;
                    plane.v_scale *= scalar;

                    if plane.u_scale <= 1. || plane.v_scale <= 1. {
                        println!(
                            "Plane at {} has low scaling [{} {}].",
                            plane.p1, plane.u_scale, plane.v_scale
                        );
                        println!("Will scale them again by scalar.");

                        plane.u_scale *= scalar;
                        plane.v_scale *= scalar;
                    }
                })
            });
        }
    });
}
