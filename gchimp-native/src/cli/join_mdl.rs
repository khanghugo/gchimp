use gchimp::modules::join_mdl::join_model;

use crate::cli::{Cli, CliRes};

pub struct JoinMdl;

impl Cli for JoinMdl {
    fn name(&self) -> &'static str {
        "join_mdl"
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

        let count = match join_model(&mut map) {
            Ok(x) => x,
            Err(err) => {
                println!("Error joining models: {err}");
                return CliRes::Err;
            }
        };

        println!("Generated {count} combined models.");

        if let Err(err) = map.write(map_path) {
            println!("Error writing map: {err}");
            return CliRes::Err;
        }

        CliRes::Ok
    }

    fn cli_help(&self) {
        std::println!(
            "\
join_mdl

<path to .map>

Check the WIKI page please
"
        );
    }
}
