use gchimp::modules::rename_texture::rename_texture;

use crate::cli::{Cli, CliRes};

pub struct RenameTexture;

impl Cli for RenameTexture {
    fn name(&self) -> &'static str {
        "rename_texture"
    }

    fn cli(&self) -> super::CliRes {
        // Example. Skips "gchimp" "<module name>". Third argument is the input
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() != 1 {
            self.cli_help();
            return CliRes::Err;
        }

        let map_path = args[0].clone();

        let Ok(mut map) = map::Map::from_file(&map_path) else {
            println!("Cannot open map file");
            return CliRes::Err;
        };

        let count = rename_texture(&mut map);

        println!("Renamed {count} faces");

        if let Err(err) = map.write(map_path) {
            println!("Error writing map: {err}");
            return CliRes::Err;
        }

        CliRes::Ok
    }

    fn cli_help(&self) {
        std::println!(
            "\
rename_texture

<path to .map>
"
        );
    }
}
