#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error occurred while parsing a CSS selector.
    #[error("Invalid CSS selector: {0}")]
    InvalidSelector(String),

    /// Error occurred while making an HTTP request.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Error occurred while performing file I/O.
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Error occurred while parsing an HTTP header name.
    #[error("Invalid HTTP header name: {0}")]
    InvalidHeaderName(#[from] reqwest::header::InvalidHeaderName),

    /// Error occurred while parsing an HTTP header value.
    #[error("Invalid HTTP header value: {0}")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    /// Error occurred while parsing a URL.
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
}

/// A specialized Result type for ripr.
pub type Result<T, E = Error> = ::std::result::Result<T, E>;
