#[derive(Debug, thiserror::Error)]
pub enum MdlError {
    #[error("Failed to parse MDL header")]
    ParseHeader,
    #[error("Failed to parse textures")]
    ParseTextures,
    #[error("Failed to parse bodyparts")]
    ParseBodyparts,
    #[error("Failed to parse bones")]
    ParseBones,
    #[error("Failed to parse bone controllers")]
    ParseBoneControllers,
    #[error("Failed to parse hitboxes")]
    ParseHitboxes,
    #[error("Failed to parse sequence groups")]
    ParseSequenceGroups,
    #[error("Failed to parse skin families")]
    ParseSkinFamilies,
    #[error("Failed to parse attachments")]
    ParseAttachments,
    #[error("Failed to parse sequences")]
    ParseSequences,
    #[error("IOError: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },
}
