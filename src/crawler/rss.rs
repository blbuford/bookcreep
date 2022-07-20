use serde::Deserialize;

use crate::model::Book;

#[derive(Debug, Deserialize, PartialEq, Default)]
pub struct Item {
    #[serde(rename = "pubDate", default)]
    pub pub_date: String,
    #[serde(rename = "book_id", default)]
    pub id: String,
    pub user_rating: usize,
    pub title: String,
    pub link: String,
    #[serde(rename = "author_name", default)]
    pub author: String,
    #[serde(rename = "book_medium_image_url", default)]
    pub image_url: String,
}

impl TryInto<Book> for &Item {
    type Error = String;

    fn try_into(self) -> Result<Book, Self::Error> {
        Book::new(
            &self.title,
            &self.link,
            &self.pub_date,
            &self.id,
            self.user_rating,
            &self.author,
            &self.image_url,
        )
        .ok_or("Unable to create Book from Item".to_string())
    }
}

#[derive(Debug, Deserialize, PartialEq, Default)]
pub struct Channel {
    pub title: String,
    #[serde(rename = "item", default)]
    pub items: Vec<Item>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Rss {
    #[serde(rename = "channel", default)]
    pub channel: Channel,
}

pub struct RssResult {
    pub rss: Rss,
    pub etag: Option<String>,
}
