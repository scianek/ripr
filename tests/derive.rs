use ripr::{html::Html, prelude::*};
use ripr_derive::Extract;

#[test]
fn test_extract_basic() {
    #[derive(Extract)]
    struct Article {
        #[extract(selector = "h1", attr = "text")]
        title: String,

        #[extract(selector = "a", attr = "href")]
        url: String,
    }

    let html =
        Html::from_str(r#"<div><h1>Test Title</h1><a href="http://test.com">Link</a></div>"#);

    let el = html.root_element();
    let article = Article::extract(el).unwrap();

    assert_eq!(article.title, "Test Title");
    assert_eq!(article.url, "http://test.com");
}

#[test]
fn test_extract_text() {
    #[derive(Extract)]
    struct Test {
        #[extract(selector = "p", attr = "text")]
        content: String,
    }

    let html = Html::from_str(r#"<p>Hello World</p>"#);

    let result = Test::extract(html.root_element()).unwrap();
    assert_eq!(result.content, "Hello World");
}
