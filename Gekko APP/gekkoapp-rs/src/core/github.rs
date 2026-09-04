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
/// `<product_id>-<target>.manifest.json` asset exists. Si el release no publica
/// ese nombre exacto se acepta el primero que termine en `-<target>.manifest.json`,
/// por compatibilidad; `validate_manifest` sigue comprobando despues que la
/// identidad del producto sea la esperada. Errors are plain strings so each
/// caller maps them onto its own availability state.
pub fn resolve_latest_release(
    repository: &str,
    product_id: &str,
    target: &str,
) -> Result<(String, String, BTreeMap<String, String>), String> {
    let url = format!("https://api.github.com/repos/{repository}/releases/latest");
    let agent = http_agent();
    let mut response = agent
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| describe_release_error(repository, &error))?;
    let body = response
        .body_mut()
        .with_config()
        .limit(MANIFEST_LIMIT_BYTES)
        .read_to_string()
        .map_err(|error| format!("respuesta invalida: {error}"))?;
    let release: GithubRelease =
        serde_json::from_str(&body).map_err(|error| format!("JSON invalido: {error}"))?;
    // Un release puede publicar manifiestos de varios productos. Se prefiere el
    // del producto que se esta resolviendo (`<product_id>-<target>.manifest.json`).
    // El id de producto no se deduce del nombre del repositorio: no siempre
    // coinciden (`KitotsuMolina/Kito-compositor` publica `kitsune-compositor`).
    let suffix = format!("-{target}.manifest.json");
    let expected = format!("{}{suffix}", product_id.to_ascii_lowercase());
    let manifest_asset = release
        .assets
        .iter()
        .find(|asset| asset.name.eq_ignore_ascii_case(&expected))
        .or_else(|| {
            // Compatibilidad: releases cuyo manifiesto no lleva el nombre del
            // repositorio. `validate_manifest` sigue comprobando despues que la
            // identidad del producto sea la esperada.
            release
                .assets
                .iter()
                .find(|asset| asset.name.ends_with(&suffix))
        })
        .ok_or_else(|| {
            format!(
                "el release {} no incluye un manifiesto para {target}",
                release.tag_name
            )
        })?;
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

/// Traduce el error de la API de GitHub a algo accionable.
///
/// El caso mas comun sin token es el limite de peticiones (403/429), que antes
/// se mostraba como un generico "release no disponible" y hacia pensar que el
/// proyecto no tenia releases publicados.
fn describe_release_error(repository: &str, error: &ureq::Error) -> String {
    let text = error.to_string();
    if text.contains("403") || text.contains("429") {
        return format!(
            "GitHub esta limitando las peticiones (repositorio {repository}). \
             Espera unos minutos y reintenta: {text}"
        );
    }
    if text.contains("404") {
        return format!("{repository} no tiene ningun release publicado todavia");
    }
    format!("release de {repository} no disponible: {text}")
}

/// Download a release manifest body with the shared agent and size cap.
pub fn download_manifest_body(url: &str) -> Result<Vec<u8>, String> {
    let agent = http_agent();
    let bytes = download_bytes(&agent, url, MANIFEST_LIMIT_BYTES)?;
    Ok(bytes)
}
