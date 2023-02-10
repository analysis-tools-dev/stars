use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Context, Result};
use futures::future::join_all;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

use crate::github::Github;
use crate::{date, STARGAZERS_PER_PAGE};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StarRecord {
    date: String,
    count: usize,
}

#[derive(Debug, Deserialize)]
struct Star {
    starred_at: String,
}

/// Get the total page count from the link header.
fn get_page_count(response: &Response) -> Result<usize> {
    let link_header = response
        .headers()
        .get("link")
        .context("No link header found. Headers: {response.headers():#?}")?;
    // Extract the last page number from the link header
    let last_page =
        regex::Regex::new(r#"next.*&page=(\d*).*last"#)?.captures(link_header.to_str()?);

    let mut page_count = 1;
    if let Some(last_page) = last_page {
        if let Some(id) = last_page.get(1) {
            page_count = id.as_str().parse()?;
        }
    }
    Ok(page_count)
}

/// Map out the request page ids based on the page count and the max requests count.
fn get_request_pages(page_count: usize, max_requests_count: usize) -> Vec<usize> {
    if page_count < max_requests_count {
        // If the page count is less than the max requests count, then
        // just add all of the pages to the request pages vector.
        (1..page_count).collect()
    } else {
        // If the page count is greater than the max requests count, then
        // calculate the request pages by dividing the page count by the
        // max requests count.
        let mut request_pages: Vec<usize> = (1..=max_requests_count)
            .into_iter()
            .map(|i| (i * page_count) / max_requests_count - 1)
            .collect();
        if !request_pages.contains(&1) {
            // If the request pages vector does not contain the page 1,
            // then add it to the request pages vector.
            request_pages.insert(0, 1);
        }
        request_pages
    }
}

#[derive(Debug)]
pub(crate) struct Crawler {
    github: Github,
    max_request_count: usize,
}

impl Display for Crawler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.github.owner)
    }
}

impl Crawler {
    pub(crate) fn new<T: Into<String>>(
        owner: T,
        repo: T,
        token: String,
        max_request_count: usize,
    ) -> Self {
        let github = Github::new(owner, repo, token);
        Self {
            github,
            max_request_count,
        }
    }

    async fn sample_star_responses(
        &self,
        request_pages: Vec<usize>,
        responses: Vec<Result<Response>>,
    ) -> Result<Vec<StarRecord>> {
        let mut star_records: Vec<StarRecord> = Vec::new();
        for (index, response) in request_pages.iter().zip(responses.into_iter()) {
            match response {
                Ok(response) => {
                    let json: Vec<Star> = response.json().await?;
                    if let Some(star) = json.get(0) {
                        let starred_at = date::iso8601_to_ymd(&star.starred_at)?;
                        star_records.push(StarRecord {
                            date: starred_at,
                            count: STARGAZERS_PER_PAGE * index,
                        });
                    }
                }
                Err(e) => {
                    println!("Error getting star record data: {e:?}");
                }
            }
        }

        Ok(star_records)
    }

    async fn parse_all_star_responses(
        &self,
        responses: Vec<Result<Response>>,
    ) -> Result<Vec<StarRecord>> {
        let mut stars: Vec<Star> = Vec::new();
        for response in responses {
            let response = response?;
            if response.status() != 200 {
                return Err(anyhow!("Response status: {}", response.status()));
            }

            let new_stars: Vec<Star> = response
                .json::<Vec<Star>>()
                .await?
                .into_iter()
                .map(|r| Star {
                    starred_at: r.starred_at,
                })
                .collect();
            stars.extend(new_stars);
        }

        let mut index = 0;
        let mut star_records: Vec<StarRecord> = Vec::new();
        while index < stars.len() {
            let starred_at = date::iso8601_to_ymd(&stars[index].starred_at)?;
            star_records.push(StarRecord {
                date: starred_at,
                count: STARGAZERS_PER_PAGE * index,
            });
            index += stars.len() / self.max_request_count;
        }
        Ok(star_records)
    }

    pub(crate) async fn stars(&self) -> Result<Vec<StarRecord>> {
        let response = self.github.stargazers(None).await?;

        // If response status is not 200, then return an error.
        if response.status() != 200 {
            return Err(anyhow!("Response status: {}", response.status()));
        }

        let page_count = get_page_count(&response)?;

        let json: Vec<Star> = response.json().await?;
        if page_count == 1 && json.is_empty() {
            // No stargazers
            return Ok(vec![]);
        }

        let request_pages = get_request_pages(page_count, self.max_request_count);
        let responses = join_all(
            request_pages
                .iter()
                .map(|page| self.github.stargazers(Some(*page))),
        )
        .await;

        let mut star_records = if request_pages.len() < self.max_request_count {
            self.parse_all_star_responses(responses).await?
        } else {
            self.sample_star_responses(request_pages, responses).await?
        };

        star_records.sort();

        let now = OffsetDateTime::now_utc();
        let add_current_stars = if star_records.is_empty() {
            true
        } else {
            let starred_at = &star_records[star_records.len() - 1].date;
            let last_date = date::parse_ymd(starred_at.as_str())?;
            (now.date() - last_date) > time::Duration::days(90)
        };

        if add_current_stars {
            let count = self.star_count().await?;
            let starred_at = date::format_ymd(now);
            star_records.push(StarRecord {
                date: starred_at,
                count,
            });
        }

        Ok(star_records)
    }

    async fn star_count(&self) -> Result<usize> {
        let data: Value = self.github.star_count().await?.json().await?;

        let value = data
            .get("stargazers_count")
            .context("No stargazers_count found")?;
        Ok(serde_json::from_value(value.clone())?)
    }
}
