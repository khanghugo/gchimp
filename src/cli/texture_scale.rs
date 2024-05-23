use super::*;

use crate::modules::texture_scale::texture_scale;

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

        let mut map = Map::new(&args[0]).unwrap();

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
