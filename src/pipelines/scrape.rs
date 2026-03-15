//! Fluent pipeline API for web scraping.

use crate::Progress;
use crate::cache::Cache;
use crate::client::{Client, ClientBuilder};
use crate::element::Element;
use crate::error::{Error, Result};
use crate::html::Html;
use crate::selection_chain::{NonEmpty, SelectionChain};
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Entry point for scraping a single URL.
///
/// Use [`scrape_many`] to scrape multiple independent URLs.
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
    pub fn pages<R>(self, range: R) -> Result<Paginated>
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

        Ok(Paginated {
            urls: (start..end)
                .map(|page| self.url.replace("{page}", &page.to_string()))
                .collect(),
            client_builder: self.client_builder,
        })
    }

    /// Select the first element matching the selector across all pages.
    pub fn select_one(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            urls: vec![self.url],
            client: self.client_builder.build()?,
            selection_chain: SelectionChain::new().select_one(selector)?,
            checkpoint: None,
            concurrency: 10,
        })
    }

    /// Select all elements matching the selector across all pages.
    pub fn select_all(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            urls: vec![self.url],
            client: self.client_builder.build()?,
            selection_chain: SelectionChain::new().select_all(selector)?,
            checkpoint: None,
            concurrency: 10,
        })
    }
}

/// Entry point for scraping multiple independent URLs.
///
/// All URLs are fetched concurrently and results are collected in order.
pub fn scrape_many<I, S>(urls: I) -> MultiScraper
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    MultiScraper {
        urls: urls.into_iter().map(Into::into).collect(),
        client_builder: Client::builder(),
    }
}

/// Scraper for multiple independent URLs.
pub struct MultiScraper {
    urls: Vec<String>,
    client_builder: ClientBuilder,
}

impl MultiScraper {
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

    /// Select the first element matching the selector across all URLs.
    pub fn select_one(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            urls: self.urls,
            client: self.client_builder.build()?,
            selection_chain: SelectionChain::new().select_one(selector)?,
            checkpoint: None,
            concurrency: 10,
        })
    }

    /// Select all elements matching the selector across all URLs.
    pub fn select_all(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            urls: self.urls,
            client: self.client_builder.build()?,
            selection_chain: SelectionChain::new().select_all(selector)?,
            checkpoint: None,
            concurrency: 10,
        })
    }
}

/// Scraper with pagination configured.
pub struct Paginated {
    urls: Vec<String>,
    client_builder: ClientBuilder,
}

impl Paginated {
    /// Select the first element matching the selector across all pages.
    pub fn select_one(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            urls: self.urls,
            client: self.client_builder.build()?,
            selection_chain: SelectionChain::new().select_one(selector)?,
            checkpoint: None,
            concurrency: 10,
        })
    }

    /// Select all elements matching the selector across all pages.
    pub fn select_all(self, selector: &str) -> Result<SelectorPipeline> {
        Ok(SelectorPipeline {
            urls: self.urls,
            client: self.client_builder.build()?,
            selection_chain: SelectionChain::new().select_all(selector)?,
            checkpoint: None,
            concurrency: 10,
        })
    }
}

/// Pipeline state after selecting elements.
pub struct SelectorPipeline {
    urls: Vec<String>,
    client: Client,
    selection_chain: SelectionChain<NonEmpty>,
    checkpoint: Option<String>,
    concurrency: usize,
}

impl SelectorPipeline {
    /// Cache fetched HTML to disk under the given name.
    ///
    /// On subsequent runs with the same URLs, the cached HTML is served from
    /// disk instead of making network requests. Cache files are stored in
    /// `.ripr-cache/`. Delete that directory to force a fresh fetch.
    pub fn checkpoint(mut self, name: &str) -> Self {
        self.checkpoint = Some(name.to_string());
        self
    }

    /// Set the number of pages fetched concurrently. Defaults to 10.
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    /// Extract an attribute value.
    pub fn attr(self, attr: &str) -> ExtractorPipeline {
        ExtractorPipeline {
            urls: self.urls,
            client: self.client,
            selection_chain: self.selection_chain,
            extractor: Extractor::Attr(attr.to_string()),
            progress_callback: None,
            checkpoint: self.checkpoint,
            concurrency: self.concurrency,
        }
    }

    /// Extract text content.
    pub fn text(self) -> ExtractorPipeline {
        ExtractorPipeline {
            urls: self.urls,
            client: self.client,
            selection_chain: self.selection_chain,
            extractor: Extractor::Text,
            progress_callback: None,
            checkpoint: self.checkpoint,
            concurrency: self.concurrency,
        }
    }

    /// Extract inner HTML.
    pub fn html(self) -> ExtractorPipeline {
        ExtractorPipeline {
            urls: self.urls,
            client: self.client,
            selection_chain: self.selection_chain,
            extractor: Extractor::Html,
            progress_callback: None,
            checkpoint: self.checkpoint,
            concurrency: self.concurrency,
        }
    }

    /// Extract structured data from each matched element using a custom type.
    ///
    /// The type must implement [`Extract`], either manually or via `#[derive(Extract)]`.
    pub fn extract<T: Extract>(self) -> CustomExtractionPipeline<T> {
        CustomExtractionPipeline::<T> {
            urls: self.urls,
            client: self.client,
            selection_chain: self.selection_chain,
            _marker: std::marker::PhantomData,
            progress_callback: None,
            checkpoint: self.checkpoint,
            concurrency: self.concurrency,
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

#[derive(Debug, Clone)]
enum Extractor {
    Attr(String),
    Text,
    Html,
}

/// Pipeline for extracting string data from selected elements.
pub struct ExtractorPipeline {
    urls: Vec<String>,
    client: Client,
    selection_chain: SelectionChain<NonEmpty>,
    extractor: Extractor,
    progress_callback: Option<Box<dyn Fn(&Progress) + Send + Sync>>,
    checkpoint: Option<String>,
    concurrency: usize,
}

impl ExtractorPipeline {
    /// Track scraping progress with a callback.
    pub fn with_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Progress) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }

    /// Collect results, failing on first error (default behavior).
    pub async fn collect(self) -> Result<Vec<String>> {
        let htmls = fetch_all_cached(
            &self.client,
            &self.urls,
            self.concurrency,
            self.checkpoint.as_deref(),
            self.progress_callback.as_deref(),
        )
        .await?;
        Ok(extract_all(htmls, &self.extractor, &self.selection_chain))
    }

    /// Collect successful results, skipping failures silently.
    pub async fn collect_ok(self) -> Vec<String> {
        let htmls = fetch_all_tolerant_cached(
            &self.client,
            &self.urls,
            self.concurrency,
            self.checkpoint.as_deref(),
            self.progress_callback.as_deref(),
        )
        .await;
        extract_all(htmls, &self.extractor, &self.selection_chain)
    }

    /// Collect with detailed error information.
    pub async fn collect_with_errors(self) -> (Vec<String>, Vec<FetchError>) {
        let (htmls, errors) = fetch_all_with_errors_cached(
            &self.client,
            &self.urls,
            self.concurrency,
            self.checkpoint.as_deref(),
            self.progress_callback.as_deref(),
        )
        .await;
        (
            extract_all(htmls, &self.extractor, &self.selection_chain),
            errors,
        )
    }
}

fn extract_all(
    htmls: Vec<Html>,
    extractor: &Extractor,
    chain: &SelectionChain<NonEmpty>,
) -> Vec<String> {
    let mut results = Vec::new();
    for html in htmls {
        let elements = html.select_chain(chain);
        let extracted: Vec<String> = match extractor {
            Extractor::Attr(attr) => elements
                .into_iter()
                .filter_map(|el| el.attr(attr).map(String::from))
                .collect(),
            Extractor::Text => elements.into_iter().map(|el| el.text()).collect(),
            Extractor::Html => elements.into_iter().map(|el| el.html()).collect(),
        };
        results.extend(extracted);
    }
    results
}

/// Pipeline for extracting typed data from selected elements.
pub struct CustomExtractionPipeline<T: Extract> {
    urls: Vec<String>,
    client: Client,
    selection_chain: SelectionChain<NonEmpty>,
    progress_callback: Option<Box<dyn Fn(&Progress) + Send + Sync>>,
    checkpoint: Option<String>,
    concurrency: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Extract> CustomExtractionPipeline<T> {
    /// Track scraping progress with a callback.
    pub fn with_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Progress) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }

    /// Collect results, failing on first error.
    pub async fn collect(self) -> Result<Vec<T>> {
        let htmls = fetch_all_cached(
            &self.client,
            &self.urls,
            self.concurrency,
            self.checkpoint.as_deref(),
            self.progress_callback.as_deref(),
        )
        .await?;
        Ok(extract_all_custom(htmls, &self.selection_chain))
    }

    /// Collect successful results, skipping failures silently.
    pub async fn collect_ok(self) -> Vec<T> {
        let htmls = fetch_all_tolerant_cached(
            &self.client,
            &self.urls,
            self.concurrency,
            self.checkpoint.as_deref(),
            self.progress_callback.as_deref(),
        )
        .await;
        extract_all_custom(htmls, &self.selection_chain)
    }

    /// Collect with detailed error information.
    pub async fn collect_with_errors(self) -> (Vec<T>, Vec<FetchError>) {
        let (htmls, errors) = fetch_all_with_errors_cached(
            &self.client,
            &self.urls,
            self.concurrency,
            self.checkpoint.as_deref(),
            self.progress_callback.as_deref(),
        )
        .await;
        (extract_all_custom(htmls, &self.selection_chain), errors)
    }
}

fn extract_all_custom<T: Extract>(htmls: Vec<Html>, chain: &SelectionChain<NonEmpty>) -> Vec<T> {
    let mut results = Vec::new();
    for html in htmls {
        let selections = html.select_chain(chain);
        results.extend(selections.into_iter().filter_map(T::extract));
    }
    results
}

/// Error information for failed fetches.
#[derive(Debug)]
pub struct FetchError {
    /// The URL that failed to fetch.
    pub url: String,
    /// The underlying error.
    pub error: Error,
}

/// Fetch all URLs, failing on first error.
async fn fetch_all(
    client: &Client,
    urls: &[String],
    concurrency: usize,
    progress_fn: Option<&(dyn Fn(&Progress) + Send + Sync)>,
) -> Result<Vec<Html>> {
    use futures::stream::{self, StreamExt, TryStreamExt};

    let total = urls.len();
    let completed = Arc::new(AtomicUsize::new(0));

    let htmls = stream::iter(urls)
        .map(|url| {
            let completed = completed.clone();
            async move {
                let result = client.fetch_html(url).await?;

                let current = completed.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(callback) = progress_fn {
                    callback(&Progress::new(current, total));
                }

                Ok::<_, Error>(result)
            }
        })
        .buffered(concurrency)
        .try_collect()
        .await?;

    Ok(htmls)
}

/// Fetch all URLs, skipping failures silently.
async fn fetch_all_tolerant(
    client: &Client,
    urls: &[String],
    concurrency: usize,
    progress_fn: Option<&(dyn Fn(&Progress) + Send + Sync)>,
) -> Vec<Html> {
    use futures::stream::{self, StreamExt};

    let total = urls.len();
    let completed = Arc::new(AtomicUsize::new(0));

    stream::iter(urls)
        .map(|url| {
            let completed = completed.clone();
            async move {
                let result = client.fetch_html(url).await.ok();

                let current = completed.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(callback) = progress_fn {
                    callback(&Progress::new(current, total));
                }

                result
            }
        })
        .buffered(concurrency)
        .filter_map(|result| async { result })
        .collect()
        .await
}

/// Fetch all URLs, collecting both successes and errors.
async fn fetch_all_with_errors(
    client: &Client,
    urls: &[String],
    concurrency: usize,
    progress_fn: Option<&(dyn Fn(&Progress) + Send + Sync)>,
) -> (Vec<Html>, Vec<FetchError>) {
    use futures::stream::{self, StreamExt};

    let total = urls.len();
    let completed = Arc::new(AtomicUsize::new(0));

    let results: Vec<_> = stream::iter(urls)
        .map(|url| {
            let url = url.clone();
            let completed = completed.clone();
            async move {
                let result = match client.fetch_html(&url).await {
                    Ok(html) => Ok(html),
                    Err(e) => Err(FetchError {
                        url: url.clone(),
                        error: e,
                    }),
                };

                let current = completed.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(callback) = progress_fn {
                    callback(&Progress::new(current, total));
                }

                result
            }
        })
        .buffered(concurrency)
        .collect()
        .await;

    let mut htmls = Vec::new();
    let mut errors = Vec::new();

    for result in results {
        match result {
            Ok(html) => htmls.push(html),
            Err(err) => errors.push(err),
        }
    }

    (htmls, errors)
}

/// Fetch all URLs with optional caching.
async fn fetch_all_cached(
    client: &Client,
    urls: &[String],
    concurrency: usize,
    checkpoint: Option<&str>,
    progress_fn: Option<&(dyn Fn(&Progress) + Send + Sync)>,
) -> Result<Vec<Html>> {
    if let Some(name) = checkpoint {
        if let Some(cached) = Cache::load(name, urls).await? {
            return Ok(cached);
        }
    }

    let htmls = fetch_all(client, urls, concurrency, progress_fn).await?;

    if let Some(name) = checkpoint {
        Cache::save(name, urls, &htmls).await?;
    }

    Ok(htmls)
}

async fn fetch_all_tolerant_cached(
    client: &Client,
    urls: &[String],
    concurrency: usize,
    checkpoint: Option<&str>,
    progress_fn: Option<&(dyn Fn(&Progress) + Send + Sync)>,
) -> Vec<Html> {
    if let Some(name) = checkpoint {
        if let Ok(Some(cached)) = Cache::load(name, urls).await {
            return cached;
        }
    }

    let htmls = fetch_all_tolerant(client, urls, concurrency, progress_fn).await;

    if let Some(name) = checkpoint {
        let _ = Cache::save(name, urls, &htmls).await;
    }

    htmls
}

async fn fetch_all_with_errors_cached(
    client: &Client,
    urls: &[String],
    concurrency: usize,
    checkpoint: Option<&str>,
    progress_fn: Option<&(dyn Fn(&Progress) + Send + Sync)>,
) -> (Vec<Html>, Vec<FetchError>) {
    if let Some(name) = checkpoint {
        if let Ok(Some(cached)) = Cache::load(name, urls).await {
            return (cached, vec![]);
        }
    }

    let result = fetch_all_with_errors(client, urls, concurrency, progress_fn).await;

    if let Some(name) = checkpoint {
        let _ = Cache::save(name, urls, &result.0).await;
    }

    result
}

/// Trait for extracting typed data from an HTML [`Element`].
///
/// Implement this manually or derive it with `#[derive(Extract)]` from the
/// `ripr-derive` crate.
///
/// # Example
///
/// ```ignore
/// use ripr_derive::Extract;
///
/// #[derive(Extract)]
/// struct Article {
///     #[extract(selector = "h1", attr = "text")]
///     title: String,
///
///     #[extract(selector = "a.read-more", attr = "href")]
///     url: String,
/// }
/// ```
pub trait Extract: Sized {
    /// Extract a value from the given element, returning `None` if extraction fails.
    fn extract(selectable: Element) -> Option<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pages_requires_placeholder() {
        let result = scrape("https://example.com/posts").pages(1..=5);
        assert!(matches!(result, Err(Error::PaginationError(_))));
    }

    #[test]
    fn test_pages_requires_bounded_start() {
        let result = scrape("https://example.com?page={page}").pages(..=5);
        assert!(matches!(result, Err(Error::PaginationError(_))));
    }

    #[test]
    fn test_pages_requires_bounded_end() {
        let result = scrape("https://example.com?page={page}").pages(1..);
        assert!(matches!(result, Err(Error::PaginationError(_))));
    }

    #[test]
    fn test_pages_start_must_be_less_than_end() {
        let result = scrape("https://example.com?page={page}").pages(5..=3);
        assert!(matches!(result, Err(Error::PaginationError(_))));
    }

    #[test]
    fn test_pages_inclusive_range_produces_correct_urls() {
        let paginated = scrape("https://example.com?page={page}")
            .pages(1..=3)
            .unwrap();
        assert_eq!(
            paginated.urls,
            vec![
                "https://example.com?page=1",
                "https://example.com?page=2",
                "https://example.com?page=3",
            ]
        );
    }

    #[test]
    fn test_pages_exclusive_range_produces_correct_urls() {
        let paginated = scrape("https://example.com?page={page}")
            .pages(1..4)
            .unwrap();
        assert_eq!(
            paginated.urls,
            vec![
                "https://example.com?page=1",
                "https://example.com?page=2",
                "https://example.com?page=3",
            ]
        );
    }
}
