use map::Map;

use crate::modules::light_scale::light_scale;

use super::Cli;

pub struct LightScale;
impl Cli for LightScale {
    fn name(&self) -> &'static str {
        "light_scale"
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
                    })
                    .unwrap()
            })
            .collect();

        let mut map = Map::from_file(&args[0]).unwrap();

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
