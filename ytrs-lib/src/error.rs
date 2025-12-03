//! Error types for ytrs

use thiserror::Error;

/// Result type alias for ytrs operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur when using ytrs
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Failed to parse JSON response
    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),

    /// YouTube API returned an error
    #[error("YouTube API error: {0}")]
    ApiError(String),

    /// Required data not found in response
    #[error("Data not found: {0}")]
    DataNotFound(String),

    /// Invalid video ID
    #[error("Invalid video ID: {0}")]
    InvalidVideoId(String),

    /// Failed to parse URL
    #[error("URL parsing failed: {0}")]
    UrlParse(#[from] url::ParseError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other errors
    #[error("{0}")]
    Other(String),

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Request error (non-reqwest)
    #[error("Request error: {0}")]
    Request(String),

    /// Cipher/decryption error
    #[error("Cipher error: {0}")]
    Cipher(String),
}
