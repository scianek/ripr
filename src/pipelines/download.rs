use crate::downloader::Downloader;
use crate::error::Result;
use crate::filename::resolve_collisions;
use crate::progress::Progress;
use crate::{client::Client, filename::filename_from_url};
use futures::stream::{self, StreamExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Entry point for downloading from URLs.
pub fn download(urls: &[String]) -> DownloadPipeline {
    DownloadPipeline {
        client: Client::default(),
        urls: urls.to_vec(),
        concurrency: 10,
    }
}

/// Pipeline for downloading URLs to disk.
pub struct DownloadPipeline {
    client: Client,
    urls: Vec<String>,
    concurrency: usize,
}

impl DownloadPipeline {
    /// Override the HTTP client used for downloads.
    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    /// Set the number of concurrent downloads. Defaults to 10.
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    /// Set the output directory. The directory is created automatically if it does not exist.
    pub fn to_dir(self, dir: impl AsRef<Path>) -> ConfiguredDownloadPipeline {
        ConfiguredDownloadPipeline {
            client: self.client,
            urls: self.urls,
            dir: dir.as_ref().to_path_buf(),
            concurrency: self.concurrency,
            naming_fn: None,
            progress_callback: None,
        }
    }
}

/// Download pipeline with output directory configured, ready to run.
pub struct ConfiguredDownloadPipeline {
    client: Client,
    urls: Vec<String>,
    dir: PathBuf,
    concurrency: usize,
    naming_fn: Option<Box<dyn Fn(usize, &str) -> String + Send + Sync>>,
    progress_callback: Option<Box<dyn Fn(&Progress) + Send + Sync>>,
}

impl ConfiguredDownloadPipeline {
    /// Customize how filenames are generated from URLs.
    pub fn name_with<F>(mut self, f: F) -> Self
    where
        F: Fn(usize, &str) -> String + Send + Sync + 'static,
    {
        self.naming_fn = Some(Box::new(f));
        self
    }

    /// Track download progress with a callback.
    pub fn with_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Progress) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }

    /// Execute all downloads concurrently and return one result per URL.
    ///
    /// Each entry in the returned `Vec` corresponds to the URL at the same index.
    /// Individual download failures return `Err` without stopping the rest -
    /// use [`Iterator::partition`] to separate successes from failures if needed.
    pub async fn run(self) -> Vec<Result<String>> {
        let naming_fn = self
            .naming_fn
            .unwrap_or_else(|| Box::new(|_idx, url| filename_from_url(url)));

        let total = self.urls.len();
        let completed = Arc::new(AtomicUsize::new(0));
        let progress_callback = self.progress_callback.map(Arc::new);

        let filenames = resolve_collisions(
            self.urls
                .iter()
                .enumerate()
                .map(|(idx, url)| naming_fn(idx, url))
                .collect(),
        );

        stream::iter(self.urls.into_iter().zip(filenames))
            .enumerate()
            .map(|(_, (url, filename))| {
                let downloader = Downloader::new(self.client.clone());
                let dir = self.dir.clone();
                let completed = completed.clone();
                let progress_callback = progress_callback.clone();

                async move {
                    let path = dir.join(filename);
                    let result = downloader.download(&url, path).await;

                    let current = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    if let Some(callback) = progress_callback.as_ref() {
                        callback(&Progress::new(current, total));
                    }

                    result
                }
            })
            .buffered(self.concurrency)
            .collect::<Vec<_>>()
            .await
    }
}
