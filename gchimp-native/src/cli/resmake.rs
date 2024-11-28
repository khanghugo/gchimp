use std::path::PathBuf;

use clap::{Parser, Subcommand};
use gchimp::modules::resmake::ResMake as ResMakeModule;

use super::*;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct ResMakeCli {
    // This is just dummy command because we are already in the command
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    // i dont want to bother with removing the hyphen so that ResMake is "resmake" but not "res-make"
    Resmake {
        /// Path to .bsp file
        #[arg(short)]
        path: PathBuf,
        /// Checks for external WADs
        #[arg(short)]
        wad_check: bool,
    },
}

pub struct ResMake;

impl Cli for ResMake {
    fn name(&self) -> &'static str {
        "resmake"
    }

    fn cli(&self) -> CliRes {
        let cli = ResMakeCli::parse();

        let Commands::Resmake { path, wad_check } = cli.command;

        let mut resmake = ResMakeModule::new();

        resmake.bsp_file(path).wad_check(wad_check);

        match resmake.single_bsp() {
            Ok(_) => CliRes::Ok,
            Err(err) => {
                println!("{}", err);
                CliRes::Err
            }
        }
    }

    fn cli_help(&self) {
        // handled by clap
        unreachable!()
    }
}
