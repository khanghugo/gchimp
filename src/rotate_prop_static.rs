use rayon::prelude::*;

use map::Map;

use crate::types::Cli;

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

pub struct RotatePropStatic;
impl Cli for RotatePropStatic {
    fn name(&self) -> &'static str {
        "rotate_prop_static"
    }

    // In, Out, New name
    fn cli(&self) {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() < 2 {
            self.cli_help();
            return;
        }

        let mut map = Map::new(&args[0]);

        rotate_prop_static(&mut map, if args.len() > 2 { Some(&args[2]) } else { None });

        map.write(&args[1]).unwrap();
    }

    fn cli_help(&self) {
        println!(
            "\
Rotate Source prop_static by +90 Z in (Y Z X)
Can optionally change prop_static to a different entity through classname

<.map> <output .map> <new prop_static classname>
"
        )
    }
}

pub trait RotatePropStaticImpl {
    fn rotate_prop_static(&mut self, rename: Option<&str>) -> &mut Self;
}

impl RotatePropStaticImpl for Map {
    fn rotate_prop_static(&mut self, rename: Option<&str>) -> &mut Self {
        rotate_prop_static(self, rename);
        self
    }
}
