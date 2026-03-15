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
