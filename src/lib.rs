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
}

pub use crate::client::Client;
pub use crate::downloader::Downloader;
pub use crate::element::Element;
pub use crate::pipelines::download::download;
pub use crate::pipelines::scrape::{Extract, scrape, scrape_many};
pub use crate::progress::Progress;
