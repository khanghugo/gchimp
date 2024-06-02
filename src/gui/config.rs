//! Parses config file

use std::{fs::OpenOptions, io::Read};

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

    Ok(config)
}
