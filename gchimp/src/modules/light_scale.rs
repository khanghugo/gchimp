use rayon::prelude::*;

use map::Map;

/// Scale (R G B Brightness) by input tuple.
///
/// Mutate the input map data.
pub fn light_scale(map: &mut Map, scalar: (f64, f64, f64, f64)) {
    map.entities.par_iter_mut().for_each(|entity| {
        let exist = entity
            .attributes
            .get("classname")
            .map(|value| value == "light" || value == "light_spot");
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

pub trait LightScaleImpl {
    fn light_scale(&mut self, scalar: (f64, f64, f64, f64)) -> &mut Self;
}

impl LightScaleImpl for Map {
    fn light_scale(&mut self, scalar: (f64, f64, f64, f64)) -> &mut Self {
        light_scale(self, scalar);
        self
    }
}
