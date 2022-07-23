use anyhow::{anyhow, Context};
use quick_xml::de::from_str;
use serenity::http::Http;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::crawler::{GovernedClient, Rss, RssResult};
use crate::discord::post_book;
use crate::model::Book;
use crate::model::User;

pub async fn crawl(http: impl AsRef<Http>, pool: Arc<SqlitePool>) -> anyhow::Result<()> {
    let client = GovernedClient::default();
    let pool = &*pool;

    loop {
        if let Some(mut users) = User::get_refreshable_users(&pool, 5).await? {
            for mut user in users.iter_mut() {
                match check_rss(&mut user, &client, "https://www.goodreads.com").await {
                    Ok(result) => {
                        if let Some(books) = result {
                            for book in books.iter() {
                                post_book(&http, &book, &user, user.get_channel_id(pool).await?)
                                    .await
                                    .context("Unable to post book to discord!")?;
                            }
                        }
                        user.update(pool)
                            .await
                            .context("unable to update timestamp in database")?;
                    }
                    Err(why) => {
                        tracing::error!(
                            error.cause_chain = ?why,
                            error.message = %why,
                            "RSS check failed because: {}",
                            why
                        );
                    }
                }
            }
        }
        sleep(Duration::from_millis(1000 * 60)).await;
    }
}
#[tracing::instrument(name = "Checking user's RSS feed", skip(client, base_uri))]
async fn check_rss(
    user: &mut User,
    client: &GovernedClient,
    base_uri: &str,
) -> anyhow::Result<Option<Vec<Book>>> {
    let url = format!(
        "{}/review/list_rss/{}?shelf=read",
        base_uri, user.goodreads_user_id
    );

    let RssResult { rss, etag } = get_rss_feed(&client, &url, &user.last_etag).await?;
    user.set_last_etag(etag);
    if let Some(last_book_id) = &user.last_book_id {
        // get items up to the last book id
        let mut book_list: Vec<Book> = Vec::new();
        for item in rss.channel.items.iter() {
            if &item.id != last_book_id {
                if let Ok(book) = item.try_into() {
                    book_list.push(book);
                }
            } else {
                break;
            }
        }
        if let Some(first) = book_list.first() {
            user.set_last_book_id(Some(first.id().to_string()));
            return Ok(Some(book_list));
        }
    } else {
        // Crawler has never run for this user
        if let Some(item) = rss.channel.items.first() {
            user.set_last_book_id(Some(item.id.to_string()));
        }
    }

    Ok(None)
}

#[tracing::instrument(name = "Retrieving RSS feed", skip(client))]
async fn get_rss_feed(
    client: &GovernedClient,
    url: &str,
    last_etag: &Option<String>,
) -> anyhow::Result<RssResult> {
    let resp = if let Some(last_etag) = last_etag {
        client.get_if_etag_modified(url, last_etag).await?
    } else {
        Some(client.get(url).await?)
    }
    .ok_or(anyhow!("ETAG is unmodified"))?;

    if resp.status().as_u16() != 200 {
        return Err(anyhow!(
            "GET request returned HTTP {}",
            resp.status().as_u16()
        ));
    }
    let etag = resp
        .headers()
        .get("etag")
        .map(|etag| etag.to_str().map(|s| s.to_owned()))
        .transpose()
        .unwrap_or(None);

    let content = resp
        .text()
        .await
        .with_context(|| "Unable to get text from response")?;

    let rss: Rss = from_str(&content).with_context(|| "Unable deserialize response")?;

    Ok(RssResult { rss, etag })
}

#[cfg(test)]
mod tests {
    use crate::crawler::crawler::{check_rss, get_rss_feed};
    use crate::crawler::{GovernedClient, RssResult};
    use crate::model::User;
    use claim::{assert_err, assert_none, assert_ok, assert_some};
    use tokio::fs::read_to_string;
    use wiremock::matchers::{any, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn get_test_data() -> String {
        read_to_string("./src/crawler/test_data/data.xml")
            .await
            .expect("Unable to read in test data")
    }

    #[tokio::test]
    async fn get_rss_feed_fails_on_unmodified_etag() {
        let client = GovernedClient::default();
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(304).insert_header("etag", "test"))
            .mount(&mock_server)
            .await;

        let err =
            assert_err!(get_rss_feed(&client, &mock_server.uri(), &Some("test".to_string())).await);
        assert!(err.to_string().contains("ETAG is unmodified"))
    }

    #[tokio::test]
    async fn get_rss_feed_fails_on_remote_server_error() {
        let client = GovernedClient::default();
        let mock_server = MockServer::start().await;

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let err =
            assert_err!(get_rss_feed(&client, &mock_server.uri(), &Some("test".to_string())).await);
        assert!(err.to_string().contains("GET request returned HTTP 500"))
    }

    #[tokio::test]
    async fn get_rss_feed_succeeds_on_valid_response_no_etag() {
        let client = GovernedClient::default();
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(get_test_data().await))
            .expect(1)
            .mount(&mock_server)
            .await;

        let RssResult { etag, .. } =
            assert_ok!(get_rss_feed(&client, &mock_server.uri(), &Some("test".to_string())).await);

        assert_none!(etag);
    }

    #[tokio::test]
    async fn get_rss_feed_succeeds_on_valid_response_with_etag() {
        let client = GovernedClient::default();
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("etag", "new-etag")
                    .set_body_string(get_test_data().await),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let RssResult { etag, .. } =
            assert_ok!(get_rss_feed(&client, &mock_server.uri(), &Some("test".to_string())).await);

        assert!(assert_some!(etag).contains("new-etag"));
    }

    #[tokio::test]
    async fn check_rss_updates_user_correctly_for_first_time_crawl() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(get_test_data().await))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = GovernedClient::default();
        let mut user = User::new(0, 0, 0, 0, None, 0, None);
        assert_none!(assert_ok!(
            check_rss(&mut user, &client, &mock_server.uri()).await
        ));
        assert_some!(user.last_book_id);
    }

    #[tokio::test]
    async fn check_rss_updates_user_last_etag_upon_etag_modification() {
        let mock_server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("etag", "new-etag")
                    .set_body_string(get_test_data().await),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = GovernedClient::default();
        let mut user = User::new(
            0,
            0,
            0,
            0,
            Some("old-etag".to_string()),
            0,
            Some("4981".to_string()),
        );
        assert_none!(assert_ok!(
            check_rss(&mut user, &client, &mock_server.uri()).await
        ));
        assert_eq!(assert_some!(user.last_etag), "new-etag");
    }

    #[tokio::test]
    async fn check_rss_returns_book_list_for_new_books_read() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("etag", "new-etag")
                    .set_body_string(get_test_data().await),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = GovernedClient::default();
        let mut user = User::new(0, 0, 0, 0, None, 0, Some("43848929".to_string()));
        let book_list = assert_some!(assert_ok!(
            check_rss(&mut user, &client, &mock_server.uri()).await
        ));
        assert_eq!(book_list.len(), 2);
        assert_eq!(assert_some!(user.last_etag), "new-etag");
        assert_eq!(assert_some!(user.last_book_id), "4981");
    }
}
