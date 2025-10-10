//! Extract data from HTML pages and elements

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
enum SelectMode {
    First(scraper::Selector),
    All(scraper::Selector),
}

#[derive(Debug, Clone)]
pub struct SelectionChain {
    levels: Vec<SelectMode>,
}

impl SelectionChain {
    pub fn new() -> Self {
        Self { levels: vec![] }
    }

    pub fn select_one(mut self, selector: &str) -> Result<Self> {
        let sel = scraper::Selector::parse(selector)
            .map_err(|e| Error::InvalidSelector(format!("{:?}", e)))?;
        self.levels.push(SelectMode::First(sel));
        Ok(self)
    }

    pub fn select_all(mut self, selector: &str) -> Result<Self> {
        let sel = scraper::Selector::parse(selector)
            .map_err(|e| Error::InvalidSelector(format!("{:?}", e)))?;
        self.levels.push(SelectMode::All(sel));
        Ok(self)
    }

    /// Execute the selection tree against an HTML document
    pub fn select<'a>(&self, html: &'a scraper::Html) -> Vec<scraper::ElementRef<'a>> {
        let mut elements = vec![html.root_element()];

        for level in &self.levels {
            elements = match level {
                SelectMode::First(selector) => elements
                    .into_iter()
                    .filter_map(|el| el.select(selector).next())
                    .collect(),
                SelectMode::All(selector) => elements
                    .into_iter()
                    .flat_map(|el| el.select(selector))
                    .collect(),
            };
        }

        elements
    }

    /// Extract values from the selected elements
    pub fn extract(&self, html: &scraper::Html, extractor: &Extractor) -> Vec<String> {
        let elements = self.select(html);

        match extractor {
            Extractor::Attr(attr) => elements
                .into_iter()
                .filter_map(|el| el.value().attr(attr))
                .map(String::from)
                .collect(),
            Extractor::Text => elements
                .into_iter()
                .map(|el| el.text().collect::<String>())
                .collect(),
            Extractor::Html => elements.into_iter().map(|el| el.html()).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Extractor {
    Attr(String),
    Text,
    Html,
}
