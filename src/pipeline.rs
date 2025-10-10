//! Fluent pipeline API for web scraping.

use crate::client::Client;
use crate::downloader::Downloader;
use crate::error::{Error, Result};
use crate::selection_chain::{Extractor, SelectionChain};
use futures::{StreamExt, stream};
use reqwest::Url;
use std::ops::{Bound, Range, RangeBounds};

/// Entry point for scraping.
pub fn scrape(url: impl Into<String>) -> ScraperPipeline {
    ScraperPipeline {
        url: url.into(),
        client: Client::new(),
    }
}

/// Pipeline for scraping a single URL, with optional pagination and headers.
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

    /// Configure pagination by specifying start and end page numbers
    pub fn pages<R>(self, range: R) -> Result<PaginatedScraper>
    where
        R: RangeBounds<usize>,
    {
        if !self.url.contains("{page}") {
            return Err(Error::PaginationError(
                "URL must contain {page} placeholder".to_owned(),
            ));
        }

        let start = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => {
                return Err(Error::PaginationError(
                    "Page range must have a start".to_owned(),
                ));
            }
        };

        let end = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => {
                return Err(Error::PaginationError(
                    "Page range must have an end".to_owned(),
                ));
            }
        };

        if start >= end {
            return Err(Error::PaginationError(
                "Start page must be less than end page".to_owned(),
            ));
        }

        Ok(PaginatedScraper {
            url: self.url,
            client: self.client,
            pagination: start..end,
        })
    }

    /// Select the first element matching the selector across all pages.
    pub fn select_one(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            url: self.url,
            client: self.client,
            pagination: None,
            selection_chain: SelectionChain::new().select_one(selector)?,
        })
    }

    /// Select all elements matching the selector across all pages.
    pub fn select_all(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            url: self.url,
            client: self.client,
            pagination: None,
            selection_chain: SelectionChain::new().select_all(selector)?,
        })
    }
}

/// Scraper with pagination configured
pub struct PaginatedScraper {
    url: String,
    client: Client,
    pagination: Range<usize>,
}

impl PaginatedScraper {
    pub fn select_one(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            url: self.url,
            client: self.client,
            pagination: Some(self.pagination),
            selection_chain: SelectionChain::new().select_one(selector)?,
        })
    }

    pub fn select_all(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            url: self.url,
            client: self.client,
            pagination: Some(self.pagination),
            selection_chain: SelectionChain::new().select_all(selector)?,
        })
    }
}

/// Pipeline state after selecting elements.
pub struct SelectorPipeline {
    url: String,
    client: Client,
    pagination: Option<Range<usize>>,
    selection_chain: SelectionChain,
}

impl SelectorPipeline {
    /// Extract an attribute value.
    pub fn attr(self, attr: &str) -> ExtractorPipeline {
        ExtractorPipeline {
            url: self.url,
            client: self.client,
            pagination: self.pagination,
            selection_chain: self.selection_chain,
            extractor: Extractor::Attr(attr.to_string()),
        }
    }

    /// Extract text content.
    pub fn text(self) -> ExtractorPipeline {
        ExtractorPipeline {
            url: self.url,
            client: self.client,
            pagination: self.pagination,
            selection_chain: self.selection_chain,
            extractor: Extractor::Text,
        }
    }

    /// Extract inner HTML.
    pub fn html(self) -> ExtractorPipeline {
        ExtractorPipeline {
            url: self.url,
            client: self.client,
            pagination: self.pagination,
            selection_chain: self.selection_chain,
            extractor: Extractor::Html,
        }
    }

    /// Narrow the selection further by selecting the first matching child.
    pub fn select_one(self, selector: &str) -> Result<Self> {
        Ok(Self {
            url: self.url,
            client: self.client,
            pagination: self.pagination,
            selection_chain: self.selection_chain.select_one(selector)?,
        })
    }

    /// Narrow the selection further by selecting all matching children.
    pub fn select_all(self, selector: &str) -> Result<Self> {
        Ok(Self {
            url: self.url,
            client: self.client,
            pagination: self.pagination,
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
    url: String,
    client: Client,
    pagination: Option<Range<usize>>,
    selection_chain: SelectionChain,
    extractor: Extractor,
}

impl ExtractorPipeline {
    /// Collect extracted values.
    pub async fn collect(self) -> Result<Vec<String>> {
        let htmls = self.fetch_paginated().await?;
        let mut results = Vec::new();
        for html in htmls {
            results.extend(self.selection_chain.extract(&html, &self.extractor));
        }
        Ok(results)
    }

    /// Download extracted URLs to a directory.
    pub async fn download_to(self, dir: &str) -> Result<Vec<Result<String>>> {
        let client = self.client.clone();
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
        // Remove {page} placeholder for base URL resolution
        let base_url_str = self.url.replace("{page}", "1");
        let base_url = Url::parse(&base_url_str)?;
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

    async fn fetch_paginated(&self) -> Result<Vec<scraper::Html>> {
        use futures::stream::{self, StreamExt, TryStreamExt};

        let page_urls: Vec<_> = if let Some(ref range) = self.pagination {
            range
                .clone()
                .map(|page| self.url.replace("{page}", &page.to_string()))
                .collect()
        } else {
            vec![self.url.clone()]
        };

        let htmls = stream::iter(page_urls)
            .map(|url| async move {
                let text = self.client.fetch_text(&url).await;
                text.map(|t| scraper::Html::parse_document(&t))
            })
            .buffer_unordered(10)
            .try_collect()
            .await?;

        Ok(htmls)
    }
}
