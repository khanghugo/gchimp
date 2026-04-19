use std::path::{Path, PathBuf};

use eyre::eyre;

pub mod parser;
mod types;
mod writer;

pub use types::*;

use crate::parser::{parse_entities, parse_map};

impl Map {
    pub fn new() -> Self {
        Self {
            tb_header: None,
            entities: vec![],
        }
    }

    pub fn from_text(text: &'_ str) -> eyre::Result<Self> {
        match parse_map(text) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(eyre!("Cannot parse text: {}", err.to_string())),
        }
    }

    pub fn from_file(path: impl AsRef<Path> + Into<PathBuf>) -> eyre::Result<Self> {
        let text = std::fs::read_to_string(path)?;

        Self::from_text(&text)
    }

    pub fn parse_entities(text: &str) -> eyre::Result<Vec<Entity>> {
        match parse_entities(text) {
            Ok((_, res)) => Ok(res),
            Err(err) => Err(eyre!("Cannot parse text: {}", err.to_string())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn file_read() {
        assert!(Map::from_file("./test/sky_vis.map").is_ok());
    }

    #[test]
    fn file_write_read() {
        let i = Map::from_file("./test/sky_vis.map").unwrap();
        i.write("./test/out/sky_vis_out.map").unwrap();

        let i = Map::from_file("./test/sky_vis.map").unwrap();
        let j = Map::from_file("./test/out/sky_vis_out.map").unwrap();

        assert_eq!(i, j);
    }

    #[test]
    fn fail_read() {
        let file = Map::from_file("./dunkin/do.nut");

        assert!(file.is_err());
    }
}
