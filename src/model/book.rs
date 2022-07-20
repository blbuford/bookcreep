use std::fmt;

use chrono::NaiveDate;

#[derive(Debug)]
pub struct Book {
    title: String,
    url: String,
    completed: NaiveDate,
    id: String,
    rating: usize,
    author: String,
    image_url: String,
}

impl fmt::Display for Book {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}] - {}", self.title, self.url, self.completed)
    }
}

impl Book {
    pub fn new(
        title: &str,
        url: &str,
        completed: &str,
        id: &str,
        rating: usize,
        author: &str,
        image_url: &str,
    ) -> Option<Self> {
        let completed_date = NaiveDate::parse_from_str(&completed, "%a, %d %h %Y %H:%M:%S %z");
        dbg!(&completed_date);
        match completed_date {
            Ok(date) => Some(Book {
                title: title.to_string(),
                url: url.to_string(),
                completed: date,
                id: id.to_string(),
                rating,
                author: author.to_string(),
                image_url: image_url.to_string(),
            }),
            Err(_) => None,
        }
    }

    pub fn url(&self) -> &String {
        &self.url
    }
    pub fn title(&self) -> &String {
        &self.title
    }
    pub fn id(&self) -> &String {
        &self.id
    }
    pub fn rating(&self) -> usize {
        self.rating
    }
    pub fn author(&self) -> &String {
        &self.author
    }
    pub fn image(&self) -> &String {
        &self.image_url
    }
}
