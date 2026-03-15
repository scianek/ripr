use ripr::{html::Html, pipelines::scrape::Extract};
use ripr_derive::Extract;

fn sample_html() -> Html {
    Html::from_str(
        r#"
        <html>
        <body>
            <article class="post">
                <h2>First Post</h2>
                <p class="body">Hello world</p>
                <a href="https://example.com/1" class="read-more">Read more</a>
                <img src="a.jpg"/>
            </article>
            <article class="post">
                <h2>Second Post</h2>
                <p class="body">Goodbye world</p>
                <a href="https://example.com/2" class="read-more">Read more</a>
                <img src="b.jpg"/>
            </article>
        </body>
        </html>
    "#,
    )
}

#[test]
fn test_attr_extraction() {
    let html = sample_html();
    let srcs: Vec<_> = html
        .select_all("img")
        .into_iter()
        .filter_map(|el| el.attr("src").map(String::from))
        .collect();
    assert_eq!(srcs, vec!["a.jpg", "b.jpg"]);
}

#[test]
fn test_text_extraction() {
    let html = sample_html();
    let titles: Vec<_> = html
        .select_all("h2")
        .into_iter()
        .map(|el| el.text())
        .collect();
    assert_eq!(titles, vec!["First Post", "Second Post"]);
}

#[test]
fn test_html_extraction() {
    let html = sample_html();
    let el = html.select_one("h2").unwrap();
    assert_eq!(el.html(), "<h2>First Post</h2>");
}

#[test]
fn test_attr_returns_none_when_missing() {
    let html = sample_html();
    let el = html.select_one("h2").unwrap();
    assert!(el.attr("href").is_none());
}

#[test]
fn test_select_one_returns_first() {
    let html = sample_html();
    let el = html.select_one("h2").unwrap();
    assert_eq!(el.text(), "First Post");
}

#[test]
fn test_select_one_returns_none_on_no_match() {
    let html = sample_html();
    assert!(html.select_one(".nonexistent").is_none());
}

#[test]
fn test_select_all_returns_empty_on_no_match() {
    let html = sample_html();
    assert!(html.select_all(".nonexistent").is_empty());
}

#[test]
fn test_derive_extract() {
    #[derive(Extract, Debug, PartialEq)]
    struct Post {
        #[extract(selector = "h2", attr = "text")]
        title: String,

        #[extract(selector = "a.read-more", attr = "href")]
        url: String,
    }

    let html = sample_html();
    let post = Post::extract(html.root_element().select_one(".post").unwrap()).unwrap();
    assert_eq!(
        post,
        Post {
            title: "First Post".to_string(),
            url: "https://example.com/1".to_string(),
        }
    );
}

#[test]
fn test_derive_extract_returns_none_on_missing_element() {
    #[derive(Extract)]
    #[allow(dead_code)]
    struct Post {
        #[extract(selector = ".nonexistent", attr = "text")]
        title: String,
    }

    let html = sample_html();
    let result = Post::extract(html.root_element());
    assert!(result.is_none());
}
