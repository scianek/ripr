//! # ripr
//!
//! Declarative web scraping for extracting and downloading media.
//!
//! ## Quick Start
//!
//! ```no_run
//! use ripr::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     ripr::scrape("https://example.com")
//!         .select("img")?
//!         .attr("src")
//!         .download_to("images")
//!         .await?;
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod downloader;
pub mod element;
pub mod error;
pub(crate) mod filename;
pub mod pipeline;
pub mod selection_chain;

pub mod prelude {
    pub use crate::client::Client;
    pub use crate::downloader::Downloader;
    pub use crate::element::Element;
    pub use crate::pipeline::{Extract, scrape};
}

pub use crate::client::Client;
pub use crate::downloader::Downloader;
pub use crate::element::Element;
pub use crate::pipeline::scrape;
