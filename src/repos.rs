use anyhow::{Error, Result};
use serde_json::Value;
use std::fmt::{Display, Formatter};

pub(crate) struct Repo {
    pub(crate) owner: String,
    pub(crate) name: String,
}

impl Display for Repo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// Get the list of repositories to get stargazers for
/// Download tools JSON file from <https://raw.githubusercontent.com/analysis-tools-dev/static-analysis/master/data/api/tools.json>
///
/// Structure of the JSON file:
/// ```json
/// {
///   "toolName": {
///     "name": "toolName",
///     "source": "https://github.com/org/toolName",
///   },
///   ...
/// }
pub(crate) async fn get_repos(tools_json_url: &str) -> Result<Vec<Repo>> {
    // Download the JSON file from the URL
    let json: Value = reqwest::get(tools_json_url).await?.json().await?;

    // Parse the JSON file
    let repos = json
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Invalid JSON"))?
        .iter()
        .map(|(name, repo)| {
            let owner = repo
                .get("source")
                .and_then(serde_json::Value::as_str)
                .and_then(|source| source.split('/').nth(3))
                .ok_or_else(|| anyhow::anyhow!("Invalid source URL"))?
                .to_string();

            Ok::<Repo, Error>(Repo {
                owner,
                name: name.to_string(),
            })
        })
        .filter_map(Result::ok)
        .collect::<Vec<Repo>>();

    Ok(repos)
}
