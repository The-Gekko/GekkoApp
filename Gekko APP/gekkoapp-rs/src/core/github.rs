use crate::installer::{download_bytes, http_agent, MANIFEST_LIMIT_BYTES};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// Resolve the latest stable GitHub release of a repository for a target.
///
/// Returns `(tag, manifest_url, asset_urls)` after verifying that the
/// `<product>-<target>.manifest.json` asset exists. Errors are plain strings
/// so each caller maps them onto its own availability state.
pub fn resolve_latest_release(
    repository: &str,
    target: &str,
) -> Result<(String, String, BTreeMap<String, String>), String> {
    let url = format!("https://api.github.com/repos/{repository}/releases/latest");
    let agent = http_agent();
    let mut response = agent
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| format!("release no disponible: {error}"))?;
    let body = response
        .body_mut()
        .with_config()
        .limit(MANIFEST_LIMIT_BYTES)
        .read_to_string()
        .map_err(|error| format!("respuesta invalida: {error}"))?;
    let release: GithubRelease =
        serde_json::from_str(&body).map_err(|error| format!("JSON invalido: {error}"))?;
    let suffix = format!("-{target}.manifest.json");
    let manifest_asset = release
        .assets
        .iter()
        .find(|asset| asset.name.ends_with(&suffix))
        .ok_or_else(|| format!("el release {} no incluye {target}", release.tag_name))?;
    let asset_urls = release
        .assets
        .iter()
        .map(|asset| (asset.name.clone(), asset.browser_download_url.clone()))
        .collect();
    Ok((
        release.tag_name,
        manifest_asset.browser_download_url.clone(),
        asset_urls,
    ))
}

/// Download a release manifest body with the shared agent and size cap.
pub fn download_manifest_body(url: &str) -> Result<Vec<u8>, String> {
    let agent = http_agent();
    let bytes = download_bytes(&agent, url, MANIFEST_LIMIT_BYTES)?;
    Ok(bytes)
}
