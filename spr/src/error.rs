#[derive(Debug, thiserror::Error)]
pub enum SprError {
    #[error("Error parsing sprite: {source}")]
    NomError {
        #[source]
        source: nom::Err<nom::error::Error<Vec<u8>>>,
    },
    #[error("Error opening sprite: {source}")]
    IOError {
        #[source]
        source: std::io::Error,
    },
}
