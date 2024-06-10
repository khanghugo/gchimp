use std::path::PathBuf;

use clap::{Parser, Subcommand};
use map::Map;
use wad::Wad;

use crate::modules::check_missing_texture::check_missing_texture;

use super::Cli;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct CheckMissingTextureCli {
    // This is just dummy command because we are already in the command
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(id = "missing_texture")]
    MissingTexture {
        /// Sets path to .map
        #[arg(short, long)]
        map: PathBuf,
        /// Sets path(s) to individual .wad
        ///
        /// Could be reused mutiple times to append more .wad(s)
        #[arg(id = "wad", short, long, action = clap::ArgAction::Append)]
        wads: Vec<PathBuf>,
    },
}

pub struct CheckMissingTexture;
impl Cli for CheckMissingTexture {
    fn name(&self) -> &'static str {
        "missing_texture"
    }

    fn cli(&self) {
        let a = CheckMissingTextureCli::parse();
        let Commands::MissingTexture { map, wads } = a.command;

        let map = Map::from_file(map).unwrap();
        let wads = wads
            .iter()
            .map(|wad| Wad::from_file(wad).unwrap())
            .collect::<Vec<Wad>>();

        let missings = check_missing_texture(&map, &wads);

        if missings.is_empty() {
            println!("There is no missing textures. Good job");
        } else {
            println!("Missing textures are:");

            for missing in missings.iter() {
                println!("{missing}");
            }
        }
    }

    fn cli_help(&self) {
        unreachable!()
    }
}
