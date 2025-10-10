//! Fluent pipeline API for web scraping.

use crate::client::Client;
use crate::downloader::Downloader;
use crate::error::Result;
use crate::selection_chain::{Extractor, SelectionChain};
use futures::{StreamExt, stream};
use reqwest::Url;

/// Entry point for scraping.
pub fn scrape(url: impl Into<String>) -> ScraperPipeline {
    ScraperPipeline {
        url: url.into(),
        client: Client::new(),
    }
}

/// Initial pipeline state: configuring the HTTP client and target URL.
pub struct ScraperPipeline {
    url: String,
    client: Client,
}

impl ScraperPipeline {
    /// Load headers from a file.
    pub fn headers_from(mut self, path: &str) -> Result<Self> {
        self.client = self.client.with_headers_from(path)?;
        Ok(self)
    }

    /// Add a single header.
    pub fn header(mut self, name: &str, value: &str) -> Result<Self> {
        self.client = self.client.with_header(name, value)?;
        Ok(self)
    }

    /// Select the first element matching the selector across all pages.
    pub fn select_one(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            scraper: self,
            selection_chain: SelectionChain::new().select_one(selector)?,
        })
    }

    /// Select all elements matching the selector across all pages.
    pub fn select_all(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            scraper: self,
            selection_chain: SelectionChain::new().select_one(selector)?,
        })
    }
}

/// Pipeline state after selecting elements.
pub struct SelectorPipeline {
    scraper: ScraperPipeline,
    selection_chain: SelectionChain,
}

impl SelectorPipeline {
    /// Extract an attribute value.
    pub fn attr(self, attr: &str) -> ExtractorPipeline {
        ExtractorPipeline {
            scraper: self.scraper,
            selection_chain: self.selection_chain,
            extractor: Extractor::Attr(attr.to_string()),
        }
    }

    /// Extract text content.
    pub fn text(self) -> ExtractorPipeline {
        ExtractorPipeline {
            scraper: self.scraper,
            selection_chain: self.selection_chain,
            extractor: Extractor::Text,
        }
    }

    /// Extract inner HTML.
    pub fn html(self) -> ExtractorPipeline {
        ExtractorPipeline {
            scraper: self.scraper,
            selection_chain: self.selection_chain,
            extractor: Extractor::Html,
        }
    }

    /// Narrow the selection further by selecting the first matching child.
    pub fn select_one(self, selector: &str) -> Result<Self> {
        Ok(Self {
            scraper: self.scraper,
            selection_chain: self.selection_chain.select_one(selector)?,
        })
    }

    /// Narrow the selection further by selecting all matching children.
    pub fn select_all(self, selector: &str) -> Result<Self> {
        Ok(Self {
            scraper: self.scraper,
            selection_chain: self.selection_chain.select_all(selector)?,
        })
    }
}

/// What to extract from selected elements.
pub enum Extractor {
    Attr(String),
    Text,
    Html,
}

/// Pipeline for extracting string data from selected elements.
pub struct ExtractorPipeline {
    scraper: ScraperPipeline,
    selection_chain: SelectionChain,
    extractor: Extractor,
}

impl ExtractorPipeline {
    /// Collect extracted values.
    pub async fn collect(self) -> Result<Vec<String>> {
        let text = self.scraper.client.fetch_text(&self.scraper.url).await?;
        let html = scraper::Html::parse_document(&text);
        Ok(self.selection_chain.extract(&html, &self.extractor))
    }

    /// Download extracted URLs to a directory.
    pub async fn download_to(self, dir: &str) -> Result<Vec<Result<String>>> {
        let client = self.scraper.client.clone();
        let urls = self.extract_urls().await?;

        let downloader = Downloader::new(client);
        Ok(stream::iter(urls)
            .map(|url| {
                let downloader = downloader.clone();
                async move {
                    let filename = url.split('/').last().unwrap_or("downloaded_file");
                    let path = format!("{}/{}", dir, filename);
                    downloader.download(&url, &path).await
                }
            })
            .buffer_unordered(10)
            .collect()
            .await)
    }

    /// Resolves relative URLs against the base URL.
    async fn extract_urls(self) -> Result<Vec<String>> {
        let base_url = Url::parse(&self.scraper.url)?;
        let urls = self.collect().await?;

        let resolved_urls = urls
            .into_iter()
            .map(|url| {
                if Url::parse(&url).is_ok() {
                    url
                } else {
                    base_url
                        .join(&url)
                        .map(|u| u.to_string())
                        .unwrap_or_else(|_| url)
                }
            })
            .collect();

        Ok(resolved_urls)
    }
}
