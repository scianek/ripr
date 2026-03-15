# ripr

Declarative web scraping in Rust. Describe what you want, not how to get it.

```rust
let images = scrape("https://example.com/gallery?page={page}")
    .pages(1..=20)?
    .select_all(".post")?
    .select_one("img")?
    .attr("src")
    .collect()
    .await?;

download(&images)
    .to_dir("output")
    .run()
    .await;
```

---

## Why ripr

Most scraping code is boilerplate: fetch a page, parse the HTML, traverse elements, handle errors, repeat across pages, download files. ripr collapses that into a typed pipeline you can read top to bottom.

The API has two layers. The pipeline layer handles the common case with minimal ceremony. The building blocks (`Client`, `Downloader`, `Html`, `Element`) are fully public for when you need to go off-script.

Built on [scraper](https://github.com/causal-agent/scraper) and [reqwest](https://github.com/seanmonstar/reqwest).

---

## Design notes

A few things in here worth calling out:

**Branching selection chains** are the core insight. A flat CSS selector like `.post img` gives you all images across all posts in document order. A selection chain gives you the first image *per post*, which is almost always what you actually want:

```rust
.select_all(".post")?   // one context per post
.select_one("img")?     // first img within each context
```

**Typestate on `SelectionChain`** makes it impossible to pass an empty chain to `select_chain` at compile time. `SelectionChain<Empty>` and `SelectionChain<NonEmpty>` are distinct types; the selector methods on `Element` and `Html` only accept the latter.

**Three error strategies** let you pick the right tradeoff per use case without changing the rest of the pipeline:

```rust
.collect()               // fail on first error
.collect_ok()            // skip failures silently
.collect_with_errors()   // get both results and errors
```

**HTTP checkpointing** serializes fetched HTML to `.ripr-cache/` so you can iterate on selectors without re-fetching. Cache invalidation is URL-set-based: if the URL list changes, the cache is ignored.

---

## Features

- Fluent pipeline API with compile-time typestate
- Branching selection chains for per-container extraction
- Derive macro for mapping elements onto your own types
- Three error strategies: fail fast, skip, or collect
- Configurable concurrency on both scrape and download pipelines
- HTTP checkpointing for development iteration
- Progress callbacks at scrape and download stages
- Custom headers inline or loaded from a file

---

## Installation

```toml
[dependencies]
ripr = "0.1"

# Optional: structured extraction via derive macro
ripr-derive = "0.1"
```

---

## Usage

### Basic scraping

```rust
use ripr::prelude::*;

let srcs = scrape("https://example.com")
    .select_all("img")?
    .attr("src")
    .collect()
    .await?;
```

### Pagination

```rust
// {page} is substituted with each number in the range
let links = scrape("https://example.com/posts?p={page}")
    .pages(1..=50)?
    .select_all("a.post-link")?
    .attr("href")
    .collect()
    .await?;
```

### Branching selection

```rust
// First thumbnail from each post, not just the first post
let thumbnails = scrape("https://example.com")
    .select_all(".post")?
    .select_one(".thumbnail")?
    .attr("src")
    .collect()
    .await?;
```

### Error handling

```rust
// Fail on first error
let results = pipeline.collect().await?;

// Skip failed pages silently
let results = pipeline.collect_ok().await;

// Collect both results and a list of failures
let (results, errors) = pipeline.collect_with_errors().await;
```

### Structured extraction

```rust
use ripr::prelude::*;
use ripr_derive::Extract;

#[derive(Extract)]
struct Article {
    #[extract(selector = "h2", attr = "text")]
    title: String,

    #[extract(selector = "a.read-more", attr = "href")]
    url: String,
}

let articles = scrape("https://example.com/blog")
    .select_all("article")?
    .extract::<Article>()
    .collect()
    .await?;
```

### Downloading

```rust
download(&urls)
    .to_dir("images")
    .with_concurrency(20)
    .name_with(|idx, _url| format!("{:04}.jpg", idx))
    .with_progress(|p| println!("{}/{}", p.completed, p.total))
    .run()
    .await;
```

### Custom headers

```rust
// Inline
scrape(url)
    .header("Cookie", "session=abc123")?
    .select_all(".item")?
    // ...

// From a file (Name: Value per line, # comments supported)
scrape(url)
    .headers_from("headers.txt")?
    .select_all(".item")?
    // ...
```

### Checkpointing

```rust
// Fetched HTML is cached to .ripr-cache/my-scrape.json
// Re-running with the same URLs serves from disk instantly
scrape(url)
    .pages(1..=100)?
    .select_all(".item")?
    .checkpoint("my-scrape")
    .attr("href")
    .collect()
    .await?;
```

### Concurrency

Both the scrape and download pipelines default to 10 concurrent requests. Override with `with_concurrency`:

```rust
scrape(url)
    .pages(1..=100)?
    .select_all(".item")?
    .with_concurrency(25)
    .attr("href")
    .collect()
    .await?;

download(&urls)
    .with_concurrency(5)
    .to_dir("output")
    .run()
    .await;
```

### Using the building blocks directly

```rust
use ripr::{Client, Html, Downloader};

let client = Client::builder()
    .header("User-Agent", "ripr/0.1")?
    .timeout(Duration::from_secs(10))
    .build()?;

let html = client.fetch_html("https://example.com").await?;
let title = html.select_one("h1").map(|el| el.text());

let downloader = Downloader::new(client);
downloader.download("https://example.com/file.zip", "file.zip").await?;
```
