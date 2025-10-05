//! URL and filename manipulation utilities.

use reqwest::Url;
use sanitize_filename::sanitize;

/// Extract a safe filename from a URL.
pub fn filename_from_url(url_str: &str) -> String {
    if let Ok(url) = Url::parse(url_str) {
        if let Some(segments) = url.path_segments() {
            if let Some(filename) = segments.last() {
                if !filename.is_empty() {
                    return sanitize(filename);
                }
            }
        }
    }

    let clean = url_str.split('?').next().unwrap_or(url_str);
    let clean = clean.split('#').next().unwrap_or(clean);

    if let Some(filename) = clean.split('/').last() {
        if !filename.is_empty() {
            return sanitize(filename);
        }
    }

    "download".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_from_url() {
        assert_eq!(
            filename_from_url("https://example.com/path/image.jpg"),
            "image.jpg"
        );

        assert_eq!(
            filename_from_url("https://example.com/path/image.jpg?size=large"),
            "image.jpg"
        );

        assert_eq!(
            filename_from_url("https://example.com/my:file*.jpg"),
            "myfile.jpg"
        );
    }
}
