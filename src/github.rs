use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Request, Response,
};
use time::OffsetDateTime;

use crate::STARGAZERS_PER_PAGE;

#[derive(Debug, Clone)]
pub(crate) struct Github {
    client: reqwest::Client,
    pub(crate) owner: String,
    pub(crate) repo: String,
    token: String,
}

impl Github {
    pub(crate) fn new<T: Into<String>>(owner: T, repo: T, token: String) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            owner: owner.into(),
            repo: repo.into(),
            token,
        }
    }

    /// Get the total star count for the repo.
    pub(crate) async fn star_count(&self) -> Result<Response> {
        self.api_call(format!(
            "https://api.github.com/repos/{owner}/{repo}",
            owner = self.owner,
            repo = self.repo,
        ))
        .await
    }

    /// Get all individual stargazers for the repo on the given page.
    pub(crate) async fn stargazers(&self, page: Option<usize>) -> Result<Response> {
        let mut url = format!(
            "https://api.github.com/repos/{owner}/{repo}/stargazers?per_page={STARGAZERS_PER_PAGE}",
            owner = self.owner,
            repo = self.repo,
            STARGAZERS_PER_PAGE = STARGAZERS_PER_PAGE,
        );
        if let Some(page) = page {
            url = format!("{url}&page={page}");
        }
        self.api_call(url).await
    }

    /// Make a single request, respecting the rate limit.
    ///
    /// If we get a 429, wait for the rate limit to reset.
    /// Retry again in a loop until we get a non-rate-limited response.
    ///
    /// The `x-ratelimit-reset` header specifies the time at which the current
    /// rate limit window resets in UTC epoch seconds (e.g. `x-ratelimit-reset: 1372700873`)
    ///
    /// Use `tokio::time::sleep` to wait until the rate limit resets.
    async fn handle_rate_limit(&self, request: Request) -> Result<Response> {
        let mut response = self
            .client
            .execute(request.try_clone().context("Request can not be cloned")?)
            .await?;
        while response.status() == 429 || response.status() == 403 {
            let reset = response
                .headers()
                .get("x-ratelimit-reset")
                .ok_or_else(|| anyhow!("Missing x-ratelimit-reset header"))?
                .to_str()?
                .parse::<i64>()?;
            let reset = OffsetDateTime::from_unix_timestamp(reset)?;
            let now = OffsetDateTime::now_utc();

            // Calculate duration to wait, in seconds
            let wait = reset - now;

            println!("Rate limit exceeded, waiting until reset at {reset} in {wait}...");
            tokio::time::sleep(wait.unsigned_abs()).await;
            response = self
                .client
                .execute(request.try_clone().context("Request can not be cloned")?)
                .await?;
        }
        Ok(response)
    }

    async fn api_call(&self, url: String) -> Result<Response> {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static("application/vnd.github.v3.star+json"),
        );
        // Set user-agent, which is required by Github to avoid 403
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static("star-history"),
        );
        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("token {}", self.token))?,
        );
        println!("Calling {url}");
        let request = self.client.get(&url).headers(headers).build()?;
        self.handle_rate_limit(request).await
    }
}
