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
//!
//! // This is different from a flat selector like ".post img:first-child"
//! // which only gets the first image from the first post
//! ```

use crate::error::{Error, Result};

/// A single level in a selection chain
#[derive(Debug, Clone)]
pub(crate) enum SelectMode {
    First(scraper::Selector),
    All(scraper::Selector),
}

/// A selection chain defines a sequence of selection operations to apply to elements
#[derive(Debug, Clone)]
pub struct SelectionChain {
    pub(crate) levels: Vec<SelectMode>,
}

impl SelectionChain {
    /// Create a new empty selection chain
    pub fn new() -> Self {
        Self { levels: vec![] }
    }

    /// Add a level that selects only the first matching element.
    ///
    /// When applied, this narrows the selection to only the first element
    /// that matches the CSS selector from each element in the current set.
    ///
    /// # Arguments
    /// * `selector` - A CSS selector string (e.g., "img", ".post", "#main")
    ///
    /// # Errors
    /// Returns an error if the CSS selector is invalid
    ///
    /// # Example
    /// ```ignore
    /// // Get the first paragraph from the main element
    /// chain.select_one("#main")?.select_one("p")?
    /// ```
    pub fn select_one(mut self, selector: &str) -> Result<Self> {
        let sel = scraper::Selector::parse(selector)
            .map_err(|e| Error::InvalidSelector(format!("{:?}", e)))?;
        self.levels.push(SelectMode::First(sel));
        Ok(self)
    }

    /// Add a level that selects all matching elements.
    ///
    /// When applied, this creates a branch point - the chain will continue
    /// independently for each matched element. This is useful for selecting
    /// multiple containers and then extracting data from each.
    ///
    /// # Arguments
    /// * `selector` - A CSS selector string (e.g., "img", ".post", "#main")
    ///
    /// # Errors
    /// Returns an error if the CSS selector is invalid
    ///
    /// # Example
    /// ```ignore
    /// // Get the first image from each post (not just the first post)
    /// chain.select_all(".post")?.select_one("img")?
    /// ```
    pub fn select_all(mut self, selector: &str) -> Result<Self> {
        let sel = scraper::Selector::parse(selector)
            .map_err(|e| Error::InvalidSelector(format!("{:?}", e)))?;
        self.levels.push(SelectMode::All(sel));
        Ok(self)
    }
}

impl Default for SelectionChain {
    fn default() -> Self {
        Self::new()
    }
}
