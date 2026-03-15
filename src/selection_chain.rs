//! Selection chain builder for multi-level element selection.
//!
//! A `SelectionChain` defines a sequence of CSS selector operations that are applied
//! hierarchically to HTML elements. Each level can either select the first matching
//! element or all matching elements, allowing for complex nested selections.
//!
//! # Branching Behavior
//!
//! The key feature of selection chains is their branching behavior:
//! - `select_one()` narrows the selection to the first match at that level
//! - `select_all()` creates a branch point - subsequent selections apply to EACH matched element
//!
//! # Example
//!
//! ```ignore
//! // Select the first image from each post
//! SelectionChain::new()
//!     .select_all(".post")?      // Find all posts (creates branches)
//!     .select_one("img")?        // For EACH post, find first image
//! ```

use crate::error::{Error, Result};
use std::marker::PhantomData;

/// Typestate marker: chain has no levels yet.
pub struct Empty;

/// Typestate marker: chain has at least one level.
pub struct NonEmpty;

/// A single level in a selection chain.
#[derive(Debug, Clone)]
pub(crate) enum SelectMode {
    First(scraper::Selector),
    All(scraper::Selector),
}

/// A selection chain defines a sequence of selection operations to apply to elements.
#[derive(Debug, Clone)]
pub struct SelectionChain<State = Empty> {
    pub(crate) levels: Vec<SelectMode>,
    _state: PhantomData<State>,
}

impl SelectionChain<Empty> {
    /// Create a new empty selection chain.
    ///
    /// Call [`select_one`] or [`select_all`] to add at least one level before use.
    ///
    /// [`select_one`]: SelectionChain::select_one
    /// [`select_all`]: SelectionChain::select_all
    pub fn new() -> Self {
        Self {
            levels: vec![],
            _state: PhantomData,
        }
    }

    /// Add a level that selects the first matching element.
    ///
    /// Transitions the chain from [`Empty`] to [`NonEmpty`], making it
    /// ready for use with [`Element::select_chain`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSelector`] if the CSS selector is invalid.
    pub fn select_one(self, selector: &str) -> Result<SelectionChain<NonEmpty>> {
        let sel = scraper::Selector::parse(selector)
            .map_err(|e| Error::InvalidSelector(format!("{:?}", e)))?;
        Ok(SelectionChain {
            levels: {
                let mut l = self.levels;
                l.push(SelectMode::First(sel));
                l
            },
            _state: PhantomData,
        })
    }

    /// Add a level that selects all matching elements.
    ///
    /// Transitions the chain from [`Empty`] to [`NonEmpty`], making it
    /// ready for use with [`Element::select_chain`]. This level acts as a
    /// branch point - subsequent levels are applied independently to each
    /// matched element.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSelector`] if the CSS selector is invalid.
    pub fn select_all(self, selector: &str) -> Result<SelectionChain<NonEmpty>> {
        let sel = scraper::Selector::parse(selector)
            .map_err(|e| Error::InvalidSelector(format!("{:?}", e)))?;
        Ok(SelectionChain {
            levels: {
                let mut l = self.levels;
                l.push(SelectMode::All(sel));
                l
            },
            _state: PhantomData,
        })
    }
}

impl SelectionChain<NonEmpty> {
    /// Add a level that selects the first matching element.
    ///
    /// Narrows the current set - for each element in the set, only the
    /// first child matching the selector is kept.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSelector`] if the CSS selector is invalid.
    pub fn select_one(self, selector: &str) -> Result<SelectionChain<NonEmpty>> {
        let sel = scraper::Selector::parse(selector)
            .map_err(|e| Error::InvalidSelector(format!("{:?}", e)))?;
        Ok(SelectionChain {
            levels: {
                let mut l = self.levels;
                l.push(SelectMode::First(sel));
                l
            },
            _state: PhantomData,
        })
    }

    /// Add a level that selects all matching elements.
    ///
    /// Expands the current set - for each element in the set, all matching
    /// children are collected. Acts as a branch point for subsequent levels.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSelector`] if the CSS selector is invalid.
    pub fn select_all(self, selector: &str) -> Result<SelectionChain<NonEmpty>> {
        let sel = scraper::Selector::parse(selector)
            .map_err(|e| Error::InvalidSelector(format!("{:?}", e)))?;
        Ok(SelectionChain {
            levels: {
                let mut l = self.levels;
                l.push(SelectMode::All(sel));
                l
            },
            _state: PhantomData,
        })
    }
}

impl Default for SelectionChain<Empty> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_one_transitions_to_non_empty() {
        let chain = SelectionChain::new().select_one("div");
        assert!(chain.is_ok());
    }

    #[test]
    fn test_select_all_transitions_to_non_empty() {
        let chain = SelectionChain::new().select_all("div");
        assert!(chain.is_ok());
    }

    #[test]
    fn test_invalid_selector_returns_error() {
        let result = SelectionChain::new().select_one("<<<");
        assert!(matches!(result, Err(Error::InvalidSelector(_))));
    }

    #[test]
    fn test_chaining_multiple_levels() {
        let chain = SelectionChain::new()
            .select_all(".post")
            .unwrap()
            .select_one("img")
            .unwrap()
            .select_all("source");
        assert!(chain.is_ok());
        assert_eq!(chain.unwrap().levels.len(), 3);
    }

    #[test]
    fn test_branching_vs_flat() {
        use crate::html::Html;

        let html = Html::from_str(
            r#"
            <div class="post"><img src="a.jpg"/><img src="b.jpg"/></div>
            <div class="post"><img src="c.jpg"/><img src="d.jpg"/></div>
        "#,
        );

        // branching: select_all then select_one gives first img per post
        let chain = SelectionChain::new()
            .select_all(".post")
            .unwrap()
            .select_one("img")
            .unwrap();
        let results: Vec<_> = html
            .select_chain(&chain)
            .into_iter()
            .filter_map(|el| el.attr("src").map(String::from))
            .collect();
        assert_eq!(results, vec!["a.jpg", "c.jpg"]);

        // flat: select_all gives every img regardless of container
        let chain = SelectionChain::new().select_all("img").unwrap();
        let results: Vec<_> = html
            .select_chain(&chain)
            .into_iter()
            .filter_map(|el| el.attr("src").map(String::from))
            .collect();
        assert_eq!(results, vec!["a.jpg", "b.jpg", "c.jpg", "d.jpg"]);
    }

    #[test]
    fn test_empty_results_at_intermediate_level() {
        use crate::html::Html;

        let html = Html::from_str(r#"<div class="post"></div>"#);

        let chain = SelectionChain::new()
            .select_all(".post")
            .unwrap()
            .select_one("img")
            .unwrap();

        let results = html.select_chain(&chain);
        assert!(results.is_empty());
    }

    #[test]
    fn test_no_matches_returns_empty() {
        use crate::html::Html;

        let html = Html::from_str(r#"<div class="post"><img src="a.jpg"/></div>"#);

        let chain = SelectionChain::new().select_all(".nonexistent").unwrap();
        let results = html.select_chain(&chain);
        assert!(results.is_empty());
    }
}
