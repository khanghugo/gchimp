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
        bsp_path: Option<PathBuf>,
        /// Path to folder containing .bsp(s)
        ///
        /// This option will create resources for all .bsp(s) inside that folder
        ///
        /// You can add environment variable GCHIMP_RESMAKE_MULTITHREAD=1 to use multithreaded version of this with folder processing
        ///
        /// There is something weird about reading files with lots of threads that results in incomplete data and crash ResMake
        #[arg(short)]
        folder_path: Option<PathBuf>,
        /// Checks for external WADs
        #[arg(long, default_value_t = false)]
        wad_check: bool,
        /// Includes default resources in result
        #[arg(long, default_value_t = false)]
        include_default: bool,
        /// Skips processing over .bsp(s) with created .res
        ///
        /// Useful if you just up in some new maps and you want to update your archive without processing too much
        #[arg(long, default_value_t = false)]
        skip_created_res: bool,
    },
}

pub struct ResMake;

impl Cli for ResMake {
    fn name(&self) -> &'static str {
        "resmake"
    }

    fn cli(&self) -> CliRes {
        let cli = ResMakeCli::parse();

        let Commands::Resmake {
            bsp_path,
            folder_path,
            wad_check,
            include_default,
            skip_created_res,
        } = cli.command;

        let mut resmake = ResMakeModule::new();

        resmake
            .wad_check(wad_check)
            .include_default_resource(include_default)
            .zip(true)
            .res(true)
            .zip_ignore_missing(true)
            .skip_created_res(skip_created_res)
            .create_linked_wad(true);

        if let Some(bsp_path) = bsp_path {
            match resmake.bsp_file(bsp_path).run() {
                Ok(_) => CliRes::Ok,
                Err(err) => {
                    println!("{}", err);
                    CliRes::Err
                }
            }
        } else if let Some(folder_path) = folder_path {
            match resmake.bsp_folder(folder_path).run_folder() {
                Ok(_) => CliRes::Ok,
                Err(err) => {
                    println!("{}", err);
                    CliRes::Err
                }
            }
        } else {
            println!("Neither .bsp path or folder to a .bsp path is supplied");
            CliRes::Err
        }
    }

    fn cli_help(&self) {
        // handled by clap
        unreachable!()
    }
}
