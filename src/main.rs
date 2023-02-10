//! This is a heavily modified port of
//! <https://github.com/bytebase/star-history> to Rust

#![warn(clippy::all, clippy::pedantic)]
#![warn(
    absolute_paths_not_starting_with_crate,
    rustdoc::invalid_html_tags,
    missing_copy_implementations,
    missing_debug_implementations,
    semicolon_in_expressions_from_macros,
    unreachable_pub,
    unused_crate_dependencies,
    unused_extern_crates,
    variant_size_differences,
    clippy::missing_const_for_fn,
    clippy::manual_let_else
)]
#![deny(anonymous_parameters, macro_use_extern_crate, pointer_structural_match)]
#![deny(missing_docs)]
#![allow(clippy::module_name_repetitions)]

mod crawler;
mod date;
mod github;
mod repos;

use anyhow::{Context, Result};
use std::{collections::HashMap, env, path::PathBuf};

// Number of total requests
const MAX_REQUEST_COUNT: usize = 10;

const TOOLS_JSON_URL: &str = "https://raw.githubusercontent.com/analysis-tools-dev/static-analysis/master/data/api/tools.json";

// Number of stargazers to fetch per page
pub(crate) const STARGAZERS_PER_PAGE: usize = 30;

use crate::crawler::Crawler;

/// Save JSON to file
fn save(path: &PathBuf, json: String) -> Result<()> {
    std::fs::write(path, json).context(format!("Failed to write JSON to file {path:?}"))
}

// Main function with error handling and tokio runtime
#[tokio::main]
async fn main() -> Result<()> {
    let token = env::var("GITHUB_TOKEN")
        .context("Github token MUST be set because we crawl a lot of repos")?;

    println!("Fetching all repos from {TOOLS_JSON_URL}...");
    let repos = repos::get_repos(TOOLS_JSON_URL).await?;
    println!("Found {} repos", repos.len());

    let mut all_stars = HashMap::new();

    for repo in repos {
        println!("Fetching {repo}");
        let crawler = Crawler::new(
            repo.owner.clone(),
            repo.name.clone(),
            token.clone(),
            MAX_REQUEST_COUNT,
        );
        match crawler.stars().await {
            Ok(stars) => {
                // Optionally save the stargazers to a JSON file
                // let path = PathBuf::from(format!("output/{}.json", repo.name));
                // let json = serde_json::to_string_pretty(&stars)?;
                // save(&path, json)?;
                all_stars.insert(repo.name, stars);
            }
            Err(err) => println!("Error for {repo}: {err}"),
        }
    }

    let json = serde_json::to_string_pretty(&all_stars)?;
    save(&PathBuf::from("stars.json"), json)?;

    Ok(())
}
