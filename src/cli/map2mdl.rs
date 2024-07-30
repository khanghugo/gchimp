use std::path::PathBuf;

use super::*;

use crate::{
    config::{parse_config, Config},
    modules::map2mdl::{entity::MAP2MDL_ENTITY_NAME, Map2Mdl},
};

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

        let config = parse_config();

        if config.is_err() {
            println!("Error parsing config.toml");
            return CliRes::Err;
        }

        let Config {
            studiomdl,
            crowbar: _,
            no_vtf: _,
            #[cfg(target_os = "linux")]
                wineprefix: config_wineprefix,
            ..
        } = config.unwrap();

        #[cfg(target_os = "linux")]
        if config_wineprefix.is_none() {
            println!("No WINECONFIG provided.");
            return CliRes::Err;
        }

        let mut binding = Map2Mdl::default();
        binding
            .auto_pickup_wad(true)
            .move_to_origin(true)
            .export_texture(true)
            .studiomdl(PathBuf::from(studiomdl).as_path())
            .map(&args[0])
            .marked_entity(true);

        #[cfg(target_os = "linux")]
        binding.wineprefix(&config_wineprefix.unwrap());

        if let Err(err) = binding.work() {
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

./gchimp map2mdl <.map>
",
            MAP2MDL_ENTITY_NAME
        )
    }
}
