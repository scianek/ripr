//! A simple HTTP client for fetching content from URLs, with support for custom headers.
//!
//! # Examples
//! ```no_run
//! use ripr::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::builder()
//!         .header("User-Agent", "ripr/0.1")?
//!         .build()?;
//!     let content = client.fetch_text("https://www.example.com").await?;
//!     println!("Fetched content: {}", content);
//!     Ok(())
//! }
//! ```

use crate::{error::Result, html::Html};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;

/// A simple HTTP client for fetching content from URLs, with support for custom headers.
#[derive(Debug, Clone, Default)]
pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    /// Create a `Client` with default settings.
    ///
    /// For custom headers, timeouts, or user agents use [`Client::builder`].
    pub fn new() -> Self {
        Self::builder().build().unwrap()
    }

    /// Create a new [`ClientBuilder`].
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Fetch raw bytes from a URL.
    pub async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.inner.get(url).send().await?;
        let response = response.error_for_status()?;
        Ok(response.bytes().await?.to_vec())
    }

    /// Fetch text content from a URL.
    pub async fn fetch_text(&self, url: &str) -> Result<String> {
        let response = self.inner.get(url).send().await?;
        let response = response.error_for_status()?;
        Ok(response.text().await?)
    }

    /// Fetch and parse an HTML document from a URL.
    pub async fn fetch_html(&self, url: &str) -> Result<Html> {
        let text = self.fetch_text(url).await?;
        Ok(Html::new(scraper::Html::parse_document(&text)))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    headers: HeaderMap,
    timeout: Option<Duration>,
    user_agent: Option<String>,
}

impl ClientBuilder {
    /// Create a new [`ClientBuilder`].
    pub fn new() -> Self {
        Self {
            headers: HeaderMap::new(),
            timeout: Some(Duration::from_secs(30)),
            user_agent: None,
        }
    }

    /// Add a single header to all requests.
    pub fn header(mut self, name: &str, value: &str) -> Result<Self> {
        let name = HeaderName::from_bytes(name.as_bytes())?;
        let value = HeaderValue::from_str(value)?;
        self.headers.insert(name, value);
        Ok(self)
    }

    /// Load headers from a file.
    ///
    /// Each line should be in `Name: Value` format.
    /// Empty lines and lines starting with `#` are ignored.
    /// Lines without a `:` separator are silently skipped.
    pub fn headers_from(mut self, path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((name, value)) = line.split_once(':') {
                let name = HeaderName::from_bytes(name.trim().as_bytes())?;
                let value = HeaderValue::from_str(value.trim())?;
                self.headers.insert(name, value);
            }
        }
        Ok(self)
    }

    /// Set the request timeout. Defaults to 30 seconds.
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Set the `User-Agent` header for all requests.
    pub fn user_agent(mut self, ua: &str) -> Self {
        self.user_agent = Some(ua.to_string());
        self
    }

    /// Build the [`Client`].
    pub fn build(self) -> Result<Client> {
        let mut builder = reqwest::Client::builder().default_headers(self.headers);

        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }

        if let Some(ua) = self.user_agent {
            builder = builder.user_agent(ua);
        }

        Ok(Client {
            inner: builder.build()?,
        })
    }
}
