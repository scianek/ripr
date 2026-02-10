//! Fluent pipeline API for web scraping.

use crate::client::{Client, ClientBuilder};
use crate::element::Element;
use crate::error::{Error, Result};
use crate::selection_chain::{Extractor, SelectionChain};
use std::ops::{Bound, Range, RangeBounds};

/// Entry point for scraping.
pub fn scrape(url: impl Into<String>) -> ScraperPipeline {
    ScraperPipeline {
        url: url.into(),
        client_builder: Client::builder(),
    }
}

/// Pipeline for scraping a single URL, with optional pagination and headers.
pub struct ScraperPipeline {
    url: String,
    client_builder: ClientBuilder,
}

impl ScraperPipeline {
    /// Load headers from a file.
    pub fn headers_from(mut self, path: &str) -> Result<Self> {
        self.client_builder = self.client_builder.headers_from(path)?;
        Ok(self)
    }

    /// Add a single header.
    pub fn header(mut self, name: &str, value: &str) -> Result<Self> {
        self.client_builder = self.client_builder.header(name, value)?;
        Ok(self)
    }

    /// Configure pagination by specifying start and end page numbers.
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
            client_builder: self.client_builder,
            pagination: start..end,
        })
    }

    /// Select the first element matching the selector across all pages.
    pub fn select_one(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            url: self.url,
            client: self.client_builder.build()?,
            pagination: None,
            selection_chain: SelectionChain::new().select_one(selector)?,
        })
    }

    /// Select all elements matching the selector across all pages.
    pub fn select_all(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            url: self.url,
            client: self.client_builder.build()?,
            pagination: None,
            selection_chain: SelectionChain::new().select_all(selector)?,
        })
    }
}

/// Scraper with pagination configured.
pub struct PaginatedScraper {
    url: String,
    client_builder: ClientBuilder,
    pagination: Range<usize>,
}

impl PaginatedScraper {
    pub fn select_one(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            url: self.url,
            client: self.client_builder.build()?,
            pagination: Some(self.pagination),
            selection_chain: SelectionChain::new().select_one(selector)?,
        })
    }

    pub fn select_all(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            url: self.url,
            client: self.client_builder.build()?,
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

    /// Extract structured data from each matched element using a custom type.
    ///
    /// The type must implement [`Extract`], either manually or via `#[derive(Extract)]`.
    pub fn extract<T: Extract>(self) -> CustomExtractionPipeline<T> {
        CustomExtractionPipeline::<T> {
            url: self.url,
            client: self.client,
            pagination: self.pagination,
            selection_chain: self.selection_chain,
            _marker: std::marker::PhantomData,
        }
    }

    /// Narrow the selection further by selecting the first matching child.
    pub fn select_one(self, selector: &str) -> Result<Self> {
        Ok(Self {
            selection_chain: self.selection_chain.select_one(selector)?,
            ..self
        })
    }

    /// Narrow the selection further by selecting all matching children.
    pub fn select_all(self, selector: &str) -> Result<Self> {
        Ok(Self {
            selection_chain: self.selection_chain.select_all(selector)?,
            ..self
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

    async fn fetch_paginated(&self) -> Result<Vec<scraper::Html>> {
        fetch_pages(&self.client, &self.url, self.pagination.as_ref()).await
    }
}

/// Pipeline for extracting typed data from selected elements.
pub struct CustomExtractionPipeline<T: Extract> {
    url: String,
    client: Client,
    pagination: Option<Range<usize>>,
    selection_chain: SelectionChain,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Extract> CustomExtractionPipeline<T> {
    pub async fn collect(self) -> Result<Vec<T>> {
        let htmls = self.fetch_paginated().await?;
        let mut results = Vec::new();
        for html in htmls {
            let selections = self.selection_chain.select(&html);
            let transformed = selections
                .into_iter()
                .filter_map(T::extract)
                .collect::<Vec<_>>();
            results.extend(transformed);
        }
        Ok(results)
    }

    async fn fetch_paginated(&self) -> Result<Vec<scraper::Html>> {
        fetch_pages(&self.client, &self.url, self.pagination.as_ref()).await
    }
}

/// Shared helper for fetching pages.
async fn fetch_pages(
    client: &Client,
    url: &str,
    pagination: Option<&Range<usize>>,
) -> Result<Vec<scraper::Html>> {
    use futures::stream::{self, StreamExt, TryStreamExt};

    let page_urls: Vec<_> = if let Some(range) = pagination {
        range
            .clone()
            .map(|page| url.replace("{page}", &page.to_string()))
            .collect()
    } else {
        vec![url.to_owned()]
    };

    let htmls = stream::iter(page_urls)
        .map(|url| async move {
            let text = client.fetch_text(&url).await;
            text.map(|t| scraper::Html::parse_document(&t))
        })
        .buffered(10)
        .try_collect()
        .await?;

    Ok(htmls)
}

pub trait Extract: Sized {
    /// Extract a value from the given element, returning `None` if extraction fails.
    fn extract(selectable: Element) -> Option<Self>;
}
