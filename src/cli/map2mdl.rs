use std::path::PathBuf;

use super::*;

use crate::{
    config::{parse_config, Config},
    modules::map2mdl::{Map2Mdl, GCHIMP_MAP2MDL_ENTITY_NAME},
};

pub struct Map2MdlCli;
impl Cli for Map2MdlCli {
    fn name(&self) -> &'static str {
        "map2mdl"
    }

    // .map file
    fn cli(&self) {
        let args: Vec<String> = std::env::args().skip(2).collect();

        if args.len() != 1 {
            self.cli_help();
            return;
        }

        let config = parse_config();

        if config.is_err() {
            println!("Error parsing config.toml");
        }

        let Config {
            studiomdl,
            crowbar: _,
            no_vtf: _,
            #[cfg(target_os = "linux")]
                wineprefix: config_wineprefix,
        } = config.unwrap();

        #[cfg(target_os = "linux")]
        if config_wineprefix.is_none() {
            println!("No WINECONFIG provided.");
            return;
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
        }
    }

    fn cli_help(&self) {
        println!(
            "\
Converts {GCHIMP_MAP2MDL_ENTITY_NAME} into model. 
Better read the documentation before you do what you do.

./gchimp map2mdl <.map>
"
        )
    }
}
