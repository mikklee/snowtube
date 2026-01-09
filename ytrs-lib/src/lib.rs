//! # ytrs
//!
//! A Rust client for YouTube's private InnerTube API.
//!
//! This library provides a type-safe interface to interact with YouTube's internal API,
//! allowing you to search for videos, retrieve video information, access comments,
//! and more without requiring API keys.
//!
//! ## Example
//!
//! ```no_run
//! use ytrs_lib::InnerTube;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = InnerTube::new().await?;
//!
//!     // Search for videos
//!     let results = client.search("rust programming").await?;
//!
//!     println!("Found {} results", results.results.len());
//!
//!     Ok(())
//! }
//! ```

mod client;
mod constants;
mod convert;
mod error;
mod models;
mod parsers;
mod provider;
pub mod relative_time;
mod utils;

#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod parsers_tests;

pub use client::InnerTube;
pub use error::{Error, Result};
pub use models::*;
pub use utils::contains_asian_characters;

// Re-export all common types
pub use common::{
    // Video types
    Channel,
    ChannelConfig,
    ChannelInfo,
    // Channel provider trait
    ChannelProvider,
    ChannelTab,
    ChannelVideos,
    // Language types
    LanguageOption,
    PlatformIcon,
    ProviderError,
    SearchResults,
    SortFilter,
    Thumbnail,
    Video,
    VideoProvider,
    default_language,
    default_locale,
    // Time utilities
    format_duration,
    format_relative_time,
    get_all_languages,
    get_language_by_locale,
    parse_duration_string,
    parse_relative_time,
};
