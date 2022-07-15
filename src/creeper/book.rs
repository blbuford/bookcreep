use std::fmt;

use chrono::NaiveDate;
use scraper::{ElementRef, Html, Selector};

#[derive(Debug)]
pub struct Book {
    title: String,
    url: String,
    completed: NaiveDate,
}

impl fmt::Display for Book {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}] - {}", self.title, self.url, self.completed)
    }
}

impl Book {
    pub fn new(title: String, url: String, completed: String) -> Option<Self> {
        let completed_date = match NaiveDate::parse_from_str(&completed, "%b %d, %Y") {
            Ok(date) => Ok(date),
            Err(_) => NaiveDate::parse_from_str(&completed, "%b %Y"),
        };

        match completed_date {
            Ok(date) => Some(Book {
                title,
                url,
                completed: date,
            }),
            Err(_) => None,
        }
    }

    pub fn url(&self) -> &String {
        &self.url
    }
}

pub async fn get_books() -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let url = "https://www.goodreads.com/review/list/5835273-tim?shelf=read";
    let res = client.get(url).send().await?.text().await?;

    let t = tokio::task::spawn_blocking(move || {
        let document = Html::parse_document(&res);
        let tbody_selector = Selector::parse("tbody#booksBody").unwrap();

        document
            .select(&tbody_selector)
            .map(|tbody| {
                tbody
                    .select(&Selector::parse("tr").unwrap())
                    .filter_map(grab_book_info)
                    .map(|book| format!("https://goodreads.com{} ", book.url()))
                    .collect::<Vec<String>>()
                    .iter()
                    .fold::<String, _>("".to_string(), |cur, next| cur + next)
            })
            .fold::<String, _>("".to_string(), |cur, next| cur + &next)
    })
    .await?;

    Ok(t)
}

fn grab_book_info(element: ElementRef<'_>) -> Option<Book> {
    let title = element
        .select(&Selector::parse("td.title").unwrap())
        .next()
        .unwrap()
        .select(&Selector::parse("a").unwrap())
        .next()
        .unwrap();

    let date_read = element
        .select(&Selector::parse("span.date_read_value").unwrap())
        .next()
        .unwrap()
        .inner_html();

    return Book::new(
        title.value().attr("title").unwrap().to_string(),
        title.value().attr("href").unwrap().to_string(),
        date_read,
    );
}
