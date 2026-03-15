//! # ripr
//!
//! Declarative web scraping for extracting and downloading media.
//!
//! ## Overview
//!
//! ripr is built around a fluent pipeline API. The typical flow is:
//! 1. Start with [`scrape`] or [`scrape_many`]
//! 2. Optionally configure pagination, headers, or concurrency
//! 3. Select elements with CSS selectors
//! 4. Extract text, attributes, or structured data via [`Extract`]
//! 5. Optionally download results with [`download`]
//!
//! The lower-level [`Client`], [`Downloader`], [`Html`], and [`Element`]
//! types are also available for building custom pipelines.
//!
//! ## Quick Start
//!
//! ```no_run
//! use ripr::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let images = scrape("https://example.com?page={page}")
//!         .pages(1..=20)?
//!         .select_all("img")?
//!         .attr("src")
//!         .collect()
//!         .await?;
//!
//!     download(&images)
//!         .to_dir("images")
//!         .run()
//!         .await;
//!
//!     Ok(())
//! }
//! ```

pub(crate) mod cache;
pub mod client;
pub mod downloader;
pub mod element;
pub mod error;
pub(crate) mod filename;
pub mod html;
pub mod pipelines;
pub mod progress;
pub mod selection_chain;

pub mod prelude {
    pub use crate::client::Client;
    pub use crate::downloader::Downloader;
    pub use crate::element::Element;
    pub use crate::pipelines::download::download;
    pub use crate::pipelines::scrape::{Extract, scrape, scrape_many};
    pub use crate::progress::Progress;

    #[cfg(feature = "derive")]
    pub use ripr_derive::Extract;
}

pub use crate::client::Client;
pub use crate::downloader::Downloader;
pub use crate::element::Element;
pub use crate::pipelines::download::download;
pub use crate::pipelines::scrape::{Extract, scrape, scrape_many};
pub use crate::progress::Progress;

#[cfg(feature = "derive")]
pub use ripr_derive::Extract;
