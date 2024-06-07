//! Parses config file

// TODO move this whole thing out of GUI because CLI can benefit from this as well
use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
};

use std::env;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub studiomdl: String,
    pub crowbar: String,
    pub no_vtf: String,
    pub wineprefix: Option<String>,
}

pub static CONFIG_FILE_NAME: &str = "config.toml";

/// Parse `config.toml` in the same folder as the binary
pub fn parse_config() -> eyre::Result<Config> {
    let path = match env::current_exe() {
        Ok(path) => path.parent().unwrap().join(CONFIG_FILE_NAME),
        Err(_) => PathBuf::from(CONFIG_FILE_NAME),
    };

    parse_config_from_file(path.as_path())
}

pub fn parse_config_from_file(path: &Path) -> eyre::Result<Config> {
    let mut file = OpenOptions::new().read(true).open(path.as_os_str())?;
    let mut buffer = String::new();

    file.read_to_string(&mut buffer)?;

    let config: Config = toml::from_str(&buffer)?;

    let root = path.parent().unwrap();

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

    #[cfg(target_os = "windows")]
    let no_vtf = if no_vtf.extension().is_none() {
        no_vtf.with_extension(".exe")
    } else {
        no_vtf
    };

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
