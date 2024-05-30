use super::*;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::modules::s2g::S2GOptions;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct S2GCliStruct {
    // This is just dummy command because we are already in the command
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    S2G {
        /// Sets path to the target for conversion
        ///
        /// This could be either a .mdl file or a folder for mass conversion
        #[arg(short, long)]
        path: PathBuf,
        /// Skips decompiling (crowbar)
        #[arg(short, long)]
        decompile: bool,
        /// Skips converting .vtf to .png
        #[arg(short, long)]
        vtf: bool,
        /// Skips converting .qc and .smd
        #[arg(short, long)]
        assembly: bool,
        /// Skips compiling model (studiomdl)
        #[arg(short, long)]
        compile: bool,
        /// Continues with S2G even if there is error
        #[arg(long)]
        force: bool,
        /// WINEPREFIX
        #[arg(long)]
        #[cfg(target_os = "linux")]
        wineprefix: String,
    },
}

pub struct S2G;
impl Cli for S2G {
    fn name(&self) -> &'static str {
        "s2g"
    }

    fn cli(&self) {
        let cli = S2GCliStruct::parse();

        let Commands::S2G {
            path,
            decompile,
            vtf,
            assembly,
            compile,
            force,
            #[cfg(target_os = "linux")]
            wineprefix,
        } = cli.command;

        let mut s2g = S2GOptions::new_with_path_to_bin(path.display().to_string().as_str(), "dist");

        s2g.decompile(!decompile)
            .vtf(!vtf)
            .smd_and_qc(!assembly)
            .compile(!compile)
            .force(force);

        #[cfg(target_os = "linux")]
        s2g.set_wine_prefix(wineprefix.as_str());

        match s2g.work() {
            Err(err) => println!("{:?}", err),
            _ => {}
        };
    }

    fn cli_help(&self) {
        todo!()
    }
}
