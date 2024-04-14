use rayon::prelude::*;

use map::Map;

use crate::types::*;

/// Scale all texture by a scalar.
///
/// Mutate the input map data.
pub fn texture_scale(map: &mut Map, scalar: f64) {
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

pub struct TextureScale;
impl Cli for TextureScale {
    fn name(&self) -> &'static str {
        "texture_scale"
    }

    // In, Out, Scale
    fn cli(&self) {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() < 3 {
            self.cli_help();
            return;
        }

        let scalar = args[2].parse::<f64>();

        if scalar.is_err() {
            println!("Cannot parse scalar.");
            self.cli_help();
            return;
        }

        let mut map = Map::new(&args[0]);

        texture_scale(&mut map, scalar.unwrap());

        map.write(&args[1]).unwrap();
    }

    fn cli_help(&self) {
        println!(
            "\
Texture scale

<.map> <output .map> <scalar>
"
        )
    }
}

pub trait TextureScaleImpl {
    fn texture_scale(&mut self, scalar: f64) -> &mut Self;
}

impl TextureScaleImpl for Map {
    fn texture_scale(&mut self, scalar: f64) -> &mut Self {
        texture_scale(self, scalar);
        self
    }
}
