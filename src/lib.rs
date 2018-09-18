extern crate mdbook;
extern crate pulldown_cmark;
extern crate pulldown_cmark_to_cmark;

use std::borrow::Cow;
use mdbook::errors::{Error, Result};
use mdbook::book::{Book, BookItem, Chapter};
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use pulldown_cmark::{Event, Parser};
use pulldown_cmark::Tag::*;
use pulldown_cmark_to_cmark::fmt::cmark;

pub struct Toc;

impl Preprocessor for Toc {
    fn name(&self) -> &str {
        "toc"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let mut res = None;
        book.for_each_mut(|item: &mut BookItem| {
            if let Some(Err(_)) = res {
                return;
            }

            if let BookItem::Chapter(ref mut chapter) = *item {
                res = Some(Toc::add_toc(chapter).map(|md| {
                    chapter.content = md;
                }));
            }
        });

        res.unwrap_or(Ok(())).map(|_| book)
    }
}

fn build_toc<'a>(toc: &[(i32, Cow<'a, str>)]) -> String {
    let mut result = String::new();

    for (level, name) in toc {
        let width = 2*(level-1) as usize;
        let slug = mdbook::utils::normalize_id(&name);
        let entry = format!("{1:0$}* [{2}](#{3})\n", width, "", name, slug);
        result.push_str(&entry);
    }

    result
}

fn add_toc(content: &str) -> Result<String> {
    let mut buf = String::with_capacity(content.len());
    let mut toc_found = false;

    let mut toc_content = vec![];
    let mut current_header_level : Option<i32> = None;

    for e in Parser::new(&content) {
        if let Event::Html(html) = e {
            if html == "<!-- toc -->\n" {
                toc_found = true;
            }
            continue;
        }
        if !toc_found {
            continue;
        }

        if let Event::Start(Header(lvl)) = e {
            if lvl < 3 {
                current_header_level = Some(lvl);
            }
            continue;
        }
        if let Event::End(Header(_)) = e {
            current_header_level = None;
            continue;
        }
        if current_header_level.is_none() {
            continue;
        }

        if let Event::Text(header) = e {
            toc_content.push((current_header_level.unwrap(), header));
        }
    }

    let toc_events = build_toc(&toc_content);
    let toc_events = Parser::new(&toc_events).collect::<Vec<_>>();

    let events = Parser::new(&content).map(|e| {
        if let Event::Html(html) = e.clone() {
            if html == "<!-- toc -->\n" {
                return toc_events.clone();
            }
        }
        vec![e]
    }).flat_map(|e| e);

    cmark(events, &mut buf, None)
        .map(|_| buf)
        .map_err(|err| Error::from(format!("Markdown serialization failed: {}", err)))
}


impl Toc {
    fn add_toc(chapter: &mut Chapter) -> Result<String> {
        add_toc(&chapter.content)
    }
}

#[cfg(test)]
mod test {
    use super::add_toc;

    #[test]
    fn adds_toc() {
        let content = r#"# Chapter

<!-- toc -->

# Header 1

## Header 1.1

# Header 2

## Header 2.1

## Header 2.2

### Header 2.2.1

"#;

        let expected = r#"# Chapter

* [Header 1](#header-1)
  * [Header 1.1](#header-11)
* [Header 2](#header-2)
  * [Header 2.1](#header-21)
  * [Header 2.2](#header-22)

# Header 1

## Header 1.1

# Header 2

## Header 2.1

## Header 2.2

### Header 2.2.1"#;

        assert_eq!(expected, add_toc(content).unwrap());
    }
}