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
