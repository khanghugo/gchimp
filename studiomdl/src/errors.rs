#[derive(thiserror::Error, Debug)]
pub enum StudioMdlError {
    #[error("Missing textures: {textures:?}")]
    MissingTextures { textures: Vec<String> },
}
