use scraper::ElementRef;

use crate::selection_chain::{NonEmpty, SelectMode, SelectionChain};

/// A reference to an HTML element within a parsed document.
///
/// Obtained via [`Html::select_one`], [`Html::select_all`], or the scrape pipeline.
/// The lifetime `'a` is tied to the [`Html`] document that owns the underlying data.
#[derive(Clone)]
pub struct Element<'a> {
    inner: ElementRef<'a>,
}

impl<'a> Element<'a> {
    pub(crate) fn new(inner: ElementRef<'a>) -> Self {
        Self { inner }
    }

    
    /// Select the first child element matching the CSS selector.
    ///
    /// Returns `None` if no element matches or if the selector is invalid.
    pub fn select_one(&self, selector: &str) -> Option<Element<'a>> {
        let sel = scraper::Selector::parse(selector).ok()?;
        self.inner.select(&sel).next().map(Element::new)
    }

    /// Select all child elements matching the CSS selector.
    ///
    /// Returns an empty `Vec` if no elements match or if the selector is invalid.
    pub fn select_all(&self, selector: &str) -> Vec<Element<'a>> {
        let sel = match scraper::Selector::parse(selector) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        self.inner.select(&sel).map(Element::new).collect()
    }

    /// Select elements using a non-empty selection chain.
    pub fn select_chain(&self, chain: &SelectionChain<NonEmpty>) -> Vec<Element<'a>> {
        let mut elements = vec![self.clone()];

        for level in &chain.levels {
            elements = match level {
                SelectMode::First(selector) => elements
                    .into_iter()
                    .filter_map(|el| el.select_one_parsed(selector))
                    .collect(),
                SelectMode::All(selector) => elements
                    .into_iter()
                    .flat_map(|el| el.select_all_parsed(selector))
                    .collect(),
            };
        }

        elements
    }

    /// Get the value of an attribute by name.
    ///
    /// Returns `None` if the attribute is not present.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.inner.value().attr(name)
    }

    
    /// Get the text content of this element and all its descendants, concatenated.
    pub fn text(&self) -> String {
        self.inner.text().collect()
    }

    /// Get the inner HTML of this element as a string.
    pub fn html(&self) -> String {
        self.inner.html()
    }

    pub(crate) fn select_one_parsed(&self, selector: &scraper::Selector) -> Option<Element<'a>> {
        self.inner.select(selector).next().map(Element::new)
    }

    pub(crate) fn select_all_parsed(&self, selector: &scraper::Selector) -> Vec<Element<'a>> {
        self.inner.select(selector).map(Element::new).collect()
    }
}
