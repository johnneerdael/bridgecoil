use super::catalog::{LAUNCHERS, ToolId, select_asset, spec};
use crate::{
    apk::{self, ApkInfo},
    backend::PocError,
    signing::{self, SignatureState},
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

const MAX_TOOL_BYTES: u64 = 200 * 1024 * 1024;
const MAX_RELEASE_JSON: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub digest: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub assets: Vec<Asset>,
}
pub trait ReleaseSource {
    fn latest(&self, repo: &str) -> Result<Release, PocError>;
    fn download(&self, asset: &Asset, dest: &Path, limit: u64) -> Result<(), PocError>;
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchedTool {
    pub id: ToolId,
    pub tag: String,
    #[serde(skip)]
    pub path: PathBuf,
    pub info: ApkInfo,
    pub signer: String,
}

/// The newest published version of a launcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestLauncher {
    pub tool: ToolId,
    pub version: String,
}

/// What each launcher has published. One that can't be reached or has no build is left out, so a
/// problem with one repository doesn't hide the others.
pub fn latest_launchers(source: &dyn ReleaseSource) -> Vec<LatestLauncher> {
    LAUNCHERS
        .iter()
        .filter_map(|&tool| {
            let release = source.latest(spec(tool).repo).ok()?;
            select_asset(tool, &release.assets, &[]).ok()?;
            Some(LatestLauncher {
                tool,
                version: release.tag.trim_start_matches('v').to_owned(),
            })
        })
        .collect()
}

fn network_error() -> PocError {
    PocError::new("toolNetwork", "Could not reach GitHub for this tool.")
}
pub fn parse_release(body: &str) -> Result<Release, PocError> {
    let value: Value = serde_json::from_str(body).map_err(|_| network_error())?;
    let tag = value["tag_name"].as_str().ok_or_else(network_error)?;
    let assets = value["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|asset| {
            let url = asset["browser_download_url"].as_str()?;
            url.starts_with("https://github.com/").then(|| Asset {
                name: asset["name"].as_str().unwrap_or_default().to_owned(),
                url: url.to_owned(),
                size: asset["size"].as_u64().unwrap_or_default(),
                digest: asset["digest"]
                    .as_str()
                    .and_then(|d| d.strip_prefix("sha256:"))
                    .map(str::to_owned),
            })
        })
        .collect();
    Ok(Release {
        tag: tag.to_owned(),
        assets,
    })
}

fn sha256_of(path: &Path) -> Result<String, PocError> {
    let mut hash = Sha256::new();
    let mut file = File::open(path).map_err(|e| PocError::new("toolIo", e.to_string()))?;
    io::copy(&mut file, &mut hash).map_err(|e| PocError::new("toolIo", e.to_string()))?;
    Ok(hex::encode(hash.finalize()))
}
fn verify(id: ToolId, asset: &Asset, path: &Path) -> Result<(ApkInfo, String), PocError> {
    let tool = spec(id);
    if let Some(expected) = &asset.digest
        && !sha256_of(path)?.eq_ignore_ascii_case(expected)
    {
        return Err(PocError::new(
            "toolIntegrity",
            format!("The {} download was damaged.", tool.name),
        ));
    }
    let identity = || {
        PocError::new(
            "toolIdentity",
            format!(
                "This {} download is not signed by its publisher, so it was refused.",
                tool.name
            ),
        )
    };
    let signing = signing::inspect(path).map_err(|_| identity())?;
    let signer = signing
        .signers
        .iter()
        .find(|s| s.sha256.eq_ignore_ascii_case(tool.signer_sha256))
        .filter(|_| signing.state == SignatureState::Verified)
        .ok_or_else(identity)?;
    let info = apk::inspect(path).map_err(|_| identity())?;
    if info.package != tool.package {
        return Err(identity());
    }
    Ok((info, signer.sha256.clone()))
}

/// Downloads the newest release of a tool. A file that fails any check is deleted.
pub fn fetch_tool(
    source: &dyn ReleaseSource,
    id: ToolId,
    abis: &[String],
    dir: &Path,
) -> Result<FetchedTool, PocError> {
    let release = source.latest(spec(id).repo)?;
    let asset = select_asset(id, &release.assets, abis)?;
    fs::create_dir_all(dir).map_err(|e| PocError::new("toolIo", e.to_string()))?;
    let path = dir.join(format!("tool-{}.apk", Uuid::new_v4()));
    let checked = source
        .download(asset, &path, MAX_TOOL_BYTES)
        .and_then(|()| verify(id, asset, &path));
    match checked {
        Ok((info, signer)) => Ok(FetchedTool {
            id,
            tag: release.tag,
            path,
            info,
            signer,
        }),
        Err(e) => {
            let _ = fs::remove_file(&path);
            Err(e)
        }
    }
}

pub struct GithubReleases {
    agent: ureq::Agent,
}
impl Default for GithubReleases {
    fn default() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .timeout_global(Some(Duration::from_secs(900)))
            .user_agent("Bridgecoil")
            .build();
        Self {
            agent: config.into(),
        }
    }
}
impl ReleaseSource for GithubReleases {
    fn latest(&self, repo: &str) -> Result<Release, PocError> {
        let body = self
            .agent
            .get(format!(
                "https://api.github.com/repos/{repo}/releases/latest"
            ))
            .call()
            .map_err(|_| network_error())?
            .body_mut()
            .with_config()
            .limit(MAX_RELEASE_JSON)
            .read_to_string()
            .map_err(|_| network_error())?;
        parse_release(&body)
    }
    fn download(&self, asset: &Asset, dest: &Path, limit: u64) -> Result<(), PocError> {
        let mut response = self
            .agent
            .get(&asset.url)
            .call()
            .map_err(|_| network_error())?;
        let mut file = File::create(dest).map_err(|e| PocError::new("toolIo", e.to_string()))?;
        let mut reader = response.body_mut().with_config().limit(limit).reader();
        io::copy(&mut reader, &mut file).map_err(|_| network_error())?;
        Ok(())
    }
}
