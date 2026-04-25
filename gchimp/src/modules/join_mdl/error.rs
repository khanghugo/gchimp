use mdl::error::MdlError;

use crate::gchimp_info::GchimpInfoError;

#[derive(Debug, thiserror::Error)]
pub enum JMdlError {
    #[error("Combined model has too many textures: {len}")]
    TooManyTextures { len: usize },
    #[error("No `output` specified.")]
    NoOutput,
    #[error("Value of `output` is not a .mdl file: `{name}`")]
    OutputNotMdl { name: String },
    #[error("gchimp_jmdl MUST have targetname")]
    NoTargetName,

    #[error("gchimp_info error: {source}")]
    GchimpInfo { source: GchimpInfoError },
    #[error("MDL error: {source}")]
    MdlError { source: MdlError },
    #[error("IOError: {source}")]
    IOError { source: std::io::Error },
}

impl From<MdlError> for JMdlError {
    fn from(value: MdlError) -> Self {
        Self::MdlError { source: value }
    }
}

impl From<GchimpInfoError> for JMdlError {
    fn from(value: GchimpInfoError) -> Self {
        Self::GchimpInfo { source: value }
    }
}
