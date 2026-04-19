use crate::entity::GchimpInfoError;

#[derive(Debug, thiserror::Error)]
pub enum JMdlError {
    #[error("gchimp_info: {source}")]
    GchimpInfo { source: GchimpInfoError },
    #[error("Combined model has too many textures: {len}")]
    TooManyTextures { len: usize },
}
