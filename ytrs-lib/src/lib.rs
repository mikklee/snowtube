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
mod error;
pub mod locale_map;
mod models;
mod parsers;
pub mod relative_time;
mod utils;

#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod parsers_tests;

pub use client::InnerTube;
pub use error::{Error, Result};
pub use locale_map::{LanguageOption, get_all_languages, get_language_by_locale};
pub use models::*;
pub use relative_time::{format_relative_time, parse_relative_time};
pub use utils::{contains_asian_characters, get_hq_thumbnail_url, parse_duration_string};
