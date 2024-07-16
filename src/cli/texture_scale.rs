use super::*;

use crate::modules::texture_scale::texture_scale;

pub struct TextureScale;
impl Cli for TextureScale {
    fn name(&self) -> &'static str {
        "texture_scale"
    }

    // In, Out, Scale
    fn cli(&self) -> CliRes {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() < 3 {
            self.cli_help();
            return CliRes::Err;
        }

        let scalar = args[2].parse::<f64>();

        if scalar.is_err() {
            println!("Cannot parse scalar.");
            self.cli_help();
            return CliRes::Err;
        }

        let mut map = Map::from_file(&args[0]).unwrap();

        texture_scale(&mut map, scalar.unwrap());

        match map.write(&args[1]) {
            Ok(_) => CliRes::Ok,
            Err(err) => {
                println!("{}", err);
                CliRes::Err
            }
        }
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
