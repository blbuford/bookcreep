use anyhow::Context;
use governor::clock::DefaultClock;
use governor::middleware::NoOpMiddleware;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use nonzero_ext::*;
use reqwest::header::HeaderMap;
use reqwest::{Client, Response, StatusCode};

use std::num::NonZeroU32;

pub struct GovernedClient {
    client: Client,
    limiter: RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>,
}

impl Default for GovernedClient {
    fn default() -> Self {
        GovernedClient::new(Client::default(), nonzero!(1u32))
    }
}
impl GovernedClient {
    pub fn new(client: Client, limit_per_second: NonZeroU32) -> Self {
        Self {
            client,
            limiter: RateLimiter::direct(Quota::per_second(limit_per_second)),
        }
    }

    pub async fn get(&self, url: &str) -> anyhow::Result<Response> {
        self.limiter.until_ready().await;
        self.client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Unable to get url: {}", url))
    }

    pub async fn head(&self, url: &str, headers: Option<HeaderMap>) -> anyhow::Result<Response> {
        self.limiter.until_ready().await;
        headers
            .map_or_else(
                || self.client.head(url).send(),
                |headers| self.client.head(url).headers(headers).send(),
            )
            .await
            .with_context(|| format!("Unable to get url: {}", url))
    }

    pub async fn get_if_etag_modified(
        &self,
        url: &str,
        etag: &str,
    ) -> anyhow::Result<Option<Response>> {
        let mut header_map = HeaderMap::new();
        header_map.append(
            "If-None-Match",
            etag.parse()
                .with_context(|| format!("Unable to parse etag: {}", etag))?,
        );
        let etag_check = self
            .head(url, Some(header_map))
            .await
            .with_context(|| "unable to to HTTP HEAD the rss feed")?;

        if etag_check.status() == StatusCode::NOT_MODIFIED {
            return Ok(None);
        }
        let result = self
            .get(url)
            .await
            .with_context(|| format!("Unable to get url: {}", url))?;
        Ok(Some(result))
    }
}

#[cfg(test)]
mod tests {
    use crate::crawler::GovernedClient;
    use claim::{assert_ge, assert_le, assert_none, assert_ok, assert_some};
    use reqwest::header::HeaderMap;
    use std::time::{Duration, Instant};
    use wiremock::matchers::{any, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn client_respects_once_per_second_limit_on_head_request_with_no_headers() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;
        let client = GovernedClient::default();
        let now = Instant::now();

        let result = assert_ok!(client.head(&mock_server.uri(), None).await);
        assert_eq!(result.status().as_u16(), 200);
        let result = assert_ok!(client.head(&mock_server.uri(), None).await);
        assert_eq!(result.status().as_u16(), 200);
        assert_ge!(now.elapsed(), Duration::from_secs(1));
    }

    #[tokio::test]
    async fn client_respects_once_per_second_limit_on_head_request_with_headers() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(header("If-None-Match", "TEST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;
        let client = GovernedClient::default();
        let now = Instant::now();
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", "TEST".parse().unwrap());

        let result = assert_ok!(client.head(&mock_server.uri(), Some(headers.clone())).await);
        assert_eq!(result.status().as_u16(), 200);
        let result = assert_ok!(client.head(&mock_server.uri(), Some(headers)).await);
        assert_eq!(result.status().as_u16(), 200);
        assert_ge!(now.elapsed(), Duration::from_secs(1));
    }

    #[tokio::test]
    async fn client_respects_once_per_second_limit_on_get_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;
        let client = GovernedClient::default();
        let now = Instant::now();

        let result = assert_ok!(client.get(&mock_server.uri()).await);
        assert_eq!(result.status().as_u16(), 200);
        let result = assert_ok!(client.get(&mock_server.uri()).await);
        assert_eq!(result.status().as_u16(), 200);
        assert_ge!(now.elapsed(), Duration::from_secs(1));
    }

    #[tokio::test]
    async fn client_returns_some_on_valid_etag_get() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(header("If-None-Match", "TEST"))
            .respond_with(ResponseTemplate::new(200).insert_header("etag", "not-test"))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).insert_header("etag", "not-test"))
            .mount(&mock_server)
            .await;

        let client = GovernedClient::default();
        let now = Instant::now();
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", "TEST".parse().unwrap());

        let result = assert_some!(assert_ok!(
            client
                .get_if_etag_modified(&mock_server.uri(), "TEST")
                .await
        ));
        assert_eq!(result.status().as_u16(), 200);
        assert_ge!(now.elapsed(), Duration::from_secs(1));
    }

    #[tokio::test]
    async fn client_returns_none_on_unchanged_etag_get() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(header("If-None-Match", "TEST"))
            .respond_with(ResponseTemplate::new(304).insert_header("etag", "not-test"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = GovernedClient::default();
        let now = Instant::now();
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", "TEST".parse().unwrap());

        assert_none!(assert_ok!(
            client
                .get_if_etag_modified(&mock_server.uri(), "TEST")
                .await
        ));
        assert_le!(now.elapsed(), Duration::from_secs(1));
    }
}
