use crate::error::Result;
use crate::html::Html;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CACHE_DIR: &str = ".ripr-cache";

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    urls: Vec<String>,
    html: Vec<String>,
}

pub struct Cache;

impl Cache {
    fn path(name: &str) -> PathBuf {
        PathBuf::from(CACHE_DIR).join(format!("{}.json", name))
    }

    /// Try to load cached HTML for the given URLs.
    pub async fn load(name: &str, urls: &[String]) -> Result<Option<Vec<Html>>> {
        let path = Self::path(name);

        if !path.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(&path)?;
        let entry: CacheEntry = serde_json::from_str(&contents)?;

        if entry.urls != urls {
            return Ok(None);
        }

        let htmls = entry
            .html
            .into_iter()
            .map(|s| Html::new(scraper::Html::parse_document(&s)))
            .collect();

        Ok(Some(htmls))
    }

    /// Save fetched HTML to cache.
    pub async fn save(name: &str, urls: &[String], htmls: &[Html]) -> Result<()> {
        std::fs::create_dir_all(CACHE_DIR)?;

        let html_strings: Vec<String> = htmls.iter().map(|h| h.to_string()).collect();

        let entry = CacheEntry {
            urls: urls.to_vec(),
            html: html_strings,
        };

        let json = serde_json::to_string_pretty(&entry)?;
        std::fs::write(Self::path(name), json)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CacheGuard(&'static str);

    impl Drop for CacheGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(Cache::path(self.0));
        }
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let urls = vec!["http://test.com".to_string()];
        let result = Cache::load("test_miss_unique", &urls).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let _guard = CacheGuard("test_hit_unique");
        let urls = vec!["http://test.com".to_string()];
        let html = Html::new(scraper::Html::parse_document("<p>test</p>"));

        Cache::save("test_hit_unique", &urls, &[html])
            .await
            .unwrap();
        let loaded = Cache::load("test_hit_unique", &urls)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(loaded.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let _guard = CacheGuard("test_invalidate_unique");
        let urls1 = vec!["http://test.com/1".to_string()];
        let urls2 = vec!["http://test.com/2".to_string()];
        let html = Html::new(scraper::Html::parse_document("<p>test</p>"));

        Cache::save("test_invalidate_unique", &urls1, &[html])
            .await
            .unwrap();
        let result = Cache::load("test_invalidate_unique", &urls2).await.unwrap();

        assert!(result.is_none());
    }
}
