//! Fluent pipeline API for web scraping.

use crate::client::Client;
use crate::downloader::Downloader;
use crate::error::Result;
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

    /// Move to selection stage.
    pub fn select(self, selector: &str) -> Result<SelectorPipeline> {
        let selector = scraper::Selector::parse(selector)
            .map_err(|e| crate::error::Error::InvalidSelector(format!("{:?}", e)))?;
        Ok(SelectorPipeline {
            scraper: self,
            selector,
        })
    }
}

/// Pipeline state after selecting elements.
pub struct SelectorPipeline {
    scraper: ScraperPipeline,
    selector: scraper::Selector,
}

impl SelectorPipeline {
    /// Extract an attribute value.
    pub fn attr(self, attr: &str) -> ExtractorPipeline {
        ExtractorPipeline {
            scraper: self.scraper,
            selector: self.selector,
            extractor: Extractor::Attr(attr.to_string()),
        }
    }

    /// Extract text content.
    pub fn text(self) -> ExtractorPipeline {
        ExtractorPipeline {
            scraper: self.scraper,
            selector: self.selector,
            extractor: Extractor::Text,
        }
    }

    /// Extract inner HTML.
    pub fn html(self) -> ExtractorPipeline {
        ExtractorPipeline {
            scraper: self.scraper,
            selector: self.selector,
            extractor: Extractor::Html,
        }
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
    selector: scraper::Selector,
    extractor: Extractor,
}

impl ExtractorPipeline {
    /// Collect extracted values.
    pub async fn collect(self) -> Result<Vec<String>> {
        let text = self.scraper.client.fetch_text(&self.scraper.url).await?;
        let html = scraper::Html::parse_document(&text);

        let values = match self.extractor {
            Extractor::Attr(attr) => html
                .select(&self.selector)
                .filter_map(|el| el.attr(&attr))
                .map(String::from)
                .collect(),
            Extractor::Text => html
                .select(&self.selector)
                .map(|el| el.text().collect())
                .collect(),
            Extractor::Html => html.select(&self.selector).map(|el| el.html()).collect(),
        };

        Ok(values)
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
