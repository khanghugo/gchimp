use rayon::prelude::*;

use map::Map;

use crate::types::Cli;

/// Scale (R G B Brightness) by input tuple.
///
/// Mutate the input map data.
pub fn light_scale(map: &mut Map, scalar: (f64, f64, f64, f64)) {
    map.entities.par_iter_mut().for_each(|entity| {
        let exist = entity
            .attributes
            .get("classname")
            .map(|value| value == "light");
        if let Some(exist) = exist {
            if exist {
                let light = entity.attributes.get("_light").map(|light| {
                    light
                        .split_ascii_whitespace()
                        .map(|v| v.parse::<f64>().unwrap())
                });

                if let Some(values) = light {
                    let values: Vec<f64> = values.collect();
                    entity.attributes.insert(
                        "_light".to_string(),
                        format!(
                            "{} {} {} {}",
                            (values[0] * scalar.0).clamp(0., 255.) as i32,
                            (values[1] * scalar.1).clamp(0., 255.) as i32,
                            (values[2] * scalar.2).clamp(0., 255.) as i32,
                            (values[3] * scalar.3).max(0.) as i32
                        ),
                    );
                }
            }
        }
    });
}

pub struct LightScale;
impl Cli for LightScale {
    fn name(&self) -> &'static str {
        "texture_scale"
    }

    // In, Out, Scale
    fn cli(&self) {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() < 6 {
            self.cli_help();
            return;
        }

        let scalars: Vec<f64> = args
            .iter()
            .skip(2)
            .map(|s| {
                s.parse::<f64>()
                    .map_err(|_| {
                        println!("Cannot parse scalar.");
                        self.cli_help();
                        return;
                    })
                    .unwrap()
            })
            .collect();

        let mut map = Map::new(&args[0]);

        light_scale(&mut map, (scalars[0], scalars[1], scalars[2], scalars[3]));

        map.write(&args[1]).unwrap();
    }

    fn cli_help(&self) {
        println!(
            "\
light entity _light values scaling.

Multiplying every number in _light field with given scalars

<.map> <output .map> <R> <G> <B> <Brightness>
"
        )
    }
}
