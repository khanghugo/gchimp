use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum BspEntitiesError {
    #[error("Cannot parse all entities")]
    Parse,
}

#[derive(Debug, thiserror::Error)]
pub enum BspError {
    #[error("Cannot parse entity lump: {source}")]
    ParseEntities {
        #[source]
        source: BspEntitiesError,
    },
    #[error("Cannot parse planes")]
    ParsePlanes,
    #[error("Cannot parse textures")]
    ParseTextures,
    #[error("Cannot parse vertices")]
    ParseVertices,
    #[error("Cannot parse visibility")]
    ParseVisibility,
    #[error("Cannot parse nodes")]
    ParseNodes,
    #[error("Cannot parse texinfo")]
    ParseTexInfo,
    #[error("Cannot parse faces")]
    ParseFaces,
    #[error("Cannot parse lightmap")]
    ParseLightmap,
    #[error("Cannot parse clipnodes")]
    ParseClipNodes,
    #[error("Cannot parse leaves")]
    ParseLeaves,
    #[error("Cannot parse mark surfaces")]
    ParseMarkSurfaces,
    #[error("Cannot parse edges")]
    ParseEdges,
    #[error("Cannot parse surface edges")]
    ParseSurfEdges,
    #[error("Cannot parse models")]
    ParseModels,
    #[error("Failed to parse a lump section")]
    LumpParseError, // Generic error for the `rest` call or unhandled parsing
    #[error("Generic failture to parse with nom")]
    NomParsingError,
    #[error("Bsp version is not 30: {version}")]
    BspVersion { version: i32 },
    #[error("Cannot read file `{path}`: {source}")]
    IOError {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
}

impl BspError {
    pub fn to_result<T>(self) -> Result<T, Self> {
        Err(self)
    }
}

impl BspEntitiesError {
    pub fn to_result<T>(self) -> Result<T, Self> {
        Err(self)
    }
}
