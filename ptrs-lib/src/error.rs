//! Error types for ptrs-lib

use snafu::Snafu;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("HTTP request failed: {source}"))]
    Http { source: reqwest::Error },

    #[snafu(display("JSON parsing failed: {source}"))]
    Json { source: serde_json::Error },

    #[snafu(display("PeerTube API error: {message}"))]
    Api { message: String },

    #[snafu(display("Video not found: {id}"))]
    VideoNotFound { id: String },

    #[snafu(display("No playable video file found"))]
    NoPlayableFile,
}

impl From<reqwest::Error> for Error {
    fn from(source: reqwest::Error) -> Self {
        Error::Http { source }
    }
}

impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Error::Json { source }
    }
}

impl From<Error> for common::ProviderError {
    fn from(err: Error) -> Self {
        match err {
            Error::Http { source } => common::ProviderError::Network {
                message: source.to_string(),
            },
            Error::Json { source } => common::ProviderError::Parse {
                message: source.to_string(),
            },
            Error::Api { message } => common::ProviderError::Api { message },
            Error::VideoNotFound { id } => common::ProviderError::NotFound { message: id },
            Error::NoPlayableFile => common::ProviderError::NotFound {
                message: "No playable file".to_string(),
            },
        }
    }
}
