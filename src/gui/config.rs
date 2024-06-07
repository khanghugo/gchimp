//! Parses config file

// TODO move this whole thing out of GUI because CLI can benefit from this as well
use std::{fs::OpenOptions, io::Read, path::PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub studiomdl: String,
    pub crowbar: String,
    pub no_vtf: String,
    pub wineprefix: Option<String>,
}

pub fn parse_config(filename: &str) -> eyre::Result<Config> {
    let mut file = OpenOptions::new().read(true).open(filename)?;
    let mut buffer = String::new();

    file.read_to_string(&mut buffer)?;

    let config: Config = toml::from_str(&buffer)?;

    let binding = PathBuf::from(filename);
    let root = binding.parent().unwrap();

    let studiomdl = PathBuf::from(config.studiomdl);
    let studiomdl = if studiomdl.is_relative() {
        root.join(studiomdl)
    } else {
        studiomdl
    }
    .canonicalize()?
    .display()
    .to_string();

    let crowbar = PathBuf::from(config.crowbar);
    let crowbar = if crowbar.is_relative() {
        root.join(crowbar)
    } else {
        crowbar
    }
    .canonicalize()?
    .display()
    .to_string();

    let no_vtf = PathBuf::from(config.no_vtf);
    let no_vtf = if no_vtf.is_relative() {
        root.join(no_vtf)
    } else {
        no_vtf
    }
    .canonicalize()?
    .display()
    .to_string();

    Ok(Config {
        studiomdl,
        crowbar,
        no_vtf,
        wineprefix: config.wineprefix,
    })
}
