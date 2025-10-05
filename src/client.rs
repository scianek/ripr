//! A simple HTTP client for fetching content from URLs, with support for custom headers.
//!
//!
//! # Examples
//! ```no_run
//! use ripr::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::new()
//!         .with_header("User-Agent", "ripr/0.1")?;
//!     let content = client.fetch_text("https://www.example.com").await?;
//!     println!("Fetched content: {}", content);
//!     Ok(())
//! }
//! ```

use crate::error::Result;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

/// A simple HTTP client for fetching content from URLs, with support for custom headers.
#[derive(Debug, Clone, Default)]
pub struct Client {
    inner: reqwest::Client,
    headers: HeaderMap,
}

impl Client {
    /// Create a new client.
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::new(),
            headers: HeaderMap::new(),
        }
    }

    /// Fetch raw bytes from a URL.
    pub async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .inner
            .get(url)
            .headers(self.headers.clone())
            .send()
            .await?;
        let response = response.error_for_status()?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Fetch text content from a URL.
    pub async fn fetch_text(&self, url: &str) -> Result<String> {
        let response = self
            .inner
            .get(url)
            .headers(self.headers.clone())
            .send()
            .await?;
        Ok(response.text().await?)
    }

    /// Load headers from a file, one per line in "Name: Value" format.
    pub fn with_headers_from(mut self, path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        for line in contents.lines() {
            if let Some((name, value)) = line.split_once(':') {
                let name = HeaderName::from_bytes(name.trim().as_bytes())?;
                let value = HeaderValue::from_str(value.trim())?;
                self.headers.insert(name, value);
            }
        }
        Ok(self)
    }

    /// Add a single header.
    pub fn with_header(mut self, name: &str, value: &str) -> Result<Self> {
        let name = HeaderName::from_bytes(name.as_bytes())?;
        let value = HeaderValue::from_str(value)?;
        self.headers.insert(name, value);
        Ok(self)
    }
}
