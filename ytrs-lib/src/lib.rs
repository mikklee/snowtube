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
//! use ytrs::InnerTube;
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
mod utils;

#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod parsers_tests;

pub use client::InnerTube;
pub use error::{Error, Result};
pub use locale_map::{LanguageOption, get_all_languages};
pub use models::*;
