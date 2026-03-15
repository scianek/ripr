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

/// Generate unique filenames from a list, adding suffixes for collisions.
pub fn resolve_collisions(filenames: Vec<String>) -> Vec<String> {
    use std::collections::HashMap;

    let mut counts: HashMap<String, usize> = HashMap::new();

    filenames
        .into_iter()
        .map(|filename| {
            let count = counts.entry(filename.clone()).or_insert(0);
            *count += 1;

            if *count > 1 {
                add_suffix(&filename, *count - 1)
            } else {
                filename
            }
        })
        .collect()
}

fn add_suffix(filename: &str, number: usize) -> String {
    if let Some((name, ext)) = filename.rsplit_once('.') {
        format!("{}_{}.{}", name, number, ext)
    } else {
        format!("{}_{}", filename, number)
    }
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

    #[test]
    fn test_resolve_collisions_single_duplicate() {
        let files = vec!["file.jpg".to_string(), "file.jpg".to_string()];
        let result = resolve_collisions(files);
        assert_eq!(result, vec!["file.jpg", "file_1.jpg"]);
    }

    #[test]
    fn test_resolve_collisions_many_duplicates() {
        let files = vec![
            "file.jpg".to_string(),
            "file.jpg".to_string(),
            "file.jpg".to_string(),
        ];
        let result = resolve_collisions(files);
        assert_eq!(result, vec!["file.jpg", "file_1.jpg", "file_2.jpg"]);
    }

    #[test]
    fn test_resolve_collisions_no_extension() {
        let files = vec!["file".to_string(), "file".to_string()];
        let result = resolve_collisions(files);
        assert_eq!(result, vec!["file", "file_1"]);
    }

    #[test]
    fn test_resolve_collisions_unique_files_unchanged() {
        let files = vec!["a.jpg".to_string(), "b.jpg".to_string()];
        let result = resolve_collisions(files.clone());
        assert_eq!(result, files);
    }

    #[test]
    fn test_resolve_collisions_mixed() {
        let files = vec![
            "a.jpg".to_string(),
            "b.jpg".to_string(),
            "a.jpg".to_string(),
            "b.jpg".to_string(),
            "c.jpg".to_string(),
        ];
        let result = resolve_collisions(files);
        assert_eq!(
            result,
            vec!["a.jpg", "b.jpg", "a_1.jpg", "b_1.jpg", "c.jpg"]
        );
    }

    #[test]
    fn test_add_suffix_with_extension() {
        assert_eq!(add_suffix("image.jpg", 1), "image_1.jpg");
    }

    #[test]
    fn test_add_suffix_without_extension() {
        assert_eq!(add_suffix("image", 1), "image_1");
    }

    #[test]
    fn test_add_suffix_multiple_dots() {
        assert_eq!(add_suffix("my.image.jpg", 1), "my.image_1.jpg");
    }
}
