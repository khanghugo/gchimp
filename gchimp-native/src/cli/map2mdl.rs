use gchimp::modules::map2mdl::{convert_all_map2mdl_entities, entity::MAP2MDL_ENTITY_NAME};

use super::*;

pub struct Map2MdlCli;
impl Cli for Map2MdlCli {
    fn name(&self) -> &'static str {
        "map2mdl"
    }

    // .map file
    fn cli(&self) -> CliRes {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() != 1 {
            self.cli_help();
            return CliRes::Err;
        }

        let map_path = &args[0];

        if let Err(err) = convert_all_map2mdl_entities(map_path) {
            println!("{}", err);
            return CliRes::Err;
        }

        CliRes::Ok
    }

    fn cli_help(&self) {
        println!(
            "\
Converts {} into model.
Better read the documentation before you do what you do.
CLI usage is intended to work along with map compiling process.

./gchimp map2mdl <.map>
",
            MAP2MDL_ENTITY_NAME
        )
    }
}
