//! # ptrs-lib
//!
//! A Rust client for PeerTube's REST API.
//!
//! ## Example
//!
//! ```ignore
//! use ptrs_lib::PeerTubeClient;
//! use common::VideoProvider;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = PeerTubeClient::new("https://peertube.example.com")?;
//!
//!     // Search for videos
//!     let results = client.search("rust programming").await?;
//!     println!("Found {} results", results.results.len());
//!
//!     Ok(())
//! }
//! ```

mod client;
mod error;
mod models;

pub use client::PeerTubeClient;
pub use error::{Error, Result};
pub use models::{
    ApiChannel, ApiSearchResponse, ApiVideo, PLATFORM_NAME, StreamingPlaylist, VideoFile,
};

// Re-export common types for convenience
pub use common::{
    Channel, ProviderError, SearchResults, Thumbnail, Video, VideoProvider, format_duration,
};
