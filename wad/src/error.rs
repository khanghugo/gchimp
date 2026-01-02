#[derive(Debug, thiserror::Error)]
pub enum WadError {
    #[error("Failed to parse header")]
    ParseHeader,
    #[error("Failed to parse directory entry")]
    ParseDirectoryEntry,
    #[error("Failed to parse file entry: {entry_index}")]
    ParseFileEntry { entry_index: usize },
    #[error("Does not support parsing compressed texture")]
    CompressedTexture,
    #[error("Unknown WAD Magic: {magic:?}")]
    UnknownWadVersion { magic: Vec<u8> },
    #[error("Unknown file type: {file_type:#02x}")]
    UnknownFileType { file_type: i8 },
    #[error("Mismatched entry count. Expect ({expect}). Have ({have})")]
    MismatchedEntryCount { expect: usize, have: usize },
    #[error("{message}")]
    GenericError { message: String },
    #[error("IOError: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },
}
