use crate::Element;

/// A parsed HTML document.
///
/// Wraps the underlying HTML parser and provides element selection.
/// Obtain one via [`Html::from_str`] or through the scrape pipeline.
pub struct Html {
    inner: scraper::Html,
}

impl Html {
    pub(crate) fn new(inner: scraper::Html) -> Self {
        Self { inner }
    }

    /// Get the root element of the document.
    pub fn root_element(&self) -> Element<'_> {
        Element::new(self.inner.root_element())
    }

    /// Select a single element matching the selector.
    pub fn select_one(&self, selector: &str) -> Option<Element<'_>> {
        let element = self.root_element();
        element.select_one(selector)
    }

    /// Select all elements matching the selector.
    pub fn select_all(&self, selector: &str) -> Vec<Element<'_>> {
        let element = self.root_element();
        element.select_all(selector)
    }
}
