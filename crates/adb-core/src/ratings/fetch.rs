use super::MAX_LIST_BYTES;
use crate::backend::PocError;
use serde_json::Value;
use std::time::Duration;

const REPO: &str = "Universal-Debloater-Alliance/universal-android-debloater-next-generation";
const LIST_PATH: &str = "resources/assets/uad_lists.json";
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub sha: String,
    pub date: String,
}
pub trait ListSource {
    fn latest_commit(&self) -> Result<Commit, PocError>;
    fn download(&self, sha: &str) -> Result<String, PocError>;
}

fn network_error() -> PocError {
    PocError::new(
        "ratingsNetwork",
        "Couldn't reach GitHub for the ratings list.",
    )
}
pub fn parse_commit(body: &str) -> Result<Commit, PocError> {
    let value: Value = serde_json::from_str(body).map_err(|_| network_error())?;
    let first = value.get(0).ok_or_else(network_error)?;
    let sha = first["sha"].as_str().unwrap_or_default();
    let date = first["commit"]["committer"]["date"]
        .as_str()
        .unwrap_or_default();
    if sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) || date.is_empty() {
        return Err(network_error());
    }
    Ok(Commit {
        sha: sha.to_owned(),
        date: date.to_owned(),
    })
}
pub fn list_url(sha: &str) -> String {
    format!("https://raw.githubusercontent.com/{REPO}/{sha}/{LIST_PATH}")
}

/// The only internet access in the app. Nothing about the device is sent.
pub struct GithubSource {
    agent: ureq::Agent,
}
impl Default for GithubSource {
    fn default() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent("Bridgecoil")
            .build();
        Self {
            agent: config.into(),
        }
    }
}
impl GithubSource {
    fn get(&self, url: &str) -> Result<String, PocError> {
        self.agent
            .get(url)
            .call()
            .map_err(|_| network_error())?
            .body_mut()
            .with_config()
            .limit(MAX_LIST_BYTES as u64)
            .read_to_string()
            .map_err(|_| network_error())
    }
}
impl ListSource for GithubSource {
    fn latest_commit(&self) -> Result<Commit, PocError> {
        parse_commit(&self.get(&format!(
            "https://api.github.com/repos/{REPO}/commits?path=resources%2Fassets%2Fuad_lists.json&per_page=1"
        ))?)
    }
    fn download(&self, sha: &str) -> Result<String, PocError> {
        self.get(&list_url(sha))
    }
}
