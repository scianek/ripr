//! A simple downloader that uses the `Client` to fetch data and saves it to a specified path.
//!
//! ```no_run
//! use ripr::Downloader;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let downloader = Downloader::default();
//!     let url = "https://www.example.com/image.png";
//!     let saved_path = downloader.download(url, "downloads/image.png").await?;
//!     println!("File saved to: {}", saved_path);
//!     Ok(())
//! }
//! ```

use super::client::Client;
use crate::error::Result;
use std::path::Path;

/// A simple downloader that uses the `Client` to fetch data and saves it to a specified path.
#[derive(Debug, Clone)]
pub struct Downloader {
    client: Client,
}

impl Downloader {
    /// Create a new downloader with the given client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Download content from the specified URL and save it to the given path.
    ///
    /// Returns the path the file was saved to as a string.
    ///
    /// If the file already exists it is returned immediately without re-fetching.
    /// Note that partially downloaded files from interrupted runs will not be
    /// re-fetched - delete them manually to force a fresh download.
    ///
    /// Parent directories are created automatically if they do not exist.
    pub async fn download(&self, url: &str, path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if path.exists() {
            return Ok(path.display().to_string());
        }

        let bytes = self.client.fetch(url).await?;
        std::fs::write(path, bytes)?;

        Ok(path.display().to_string())
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new(Client::default())
    }
}
