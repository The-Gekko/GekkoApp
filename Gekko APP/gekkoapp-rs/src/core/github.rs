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

/// SHA completo (40 caracteres, en minusculas) del ultimo commit de una rama.
///
/// Es la "ultima version" de los componentes que se instalan desde una rama
/// en vez de desde un release (Gekko ADB Studio). Con `Accept:
/// application/vnd.github.sha` la API responde solo el SHA, sin el JSON del
/// commit.
pub fn resolve_branch_head(repository: &str, branch: &str) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{repository}/commits/{branch}");
    let mut response = http_agent()
        .get(&url)
        .header("Accept", "application/vnd.github.sha")
        .call()
        .map_err(|error| {
            let text = error.to_string();
            if text.contains("404") {
                format!("{repository} no tiene la rama {branch}")
            } else {
                describe_release_error(repository, &error)
            }
        })?;
    let body = response
        .body_mut()
        .with_config()
        .limit(MANIFEST_LIMIT_BYTES)
        .read_to_string()
        .map_err(|error| format!("respuesta invalida: {error}"))?;
    let sha = body.trim().to_ascii_lowercase();
    if sha.len() == 40 && sha.chars().all(|caracter| caracter.is_ascii_hexdigit()) {
        Ok(sha)
    } else {
        Err(format!(
            "respuesta inesperada al pedir el ultimo commit de {repository}/{branch}"
        ))
    }
}

/// ¿Es `recorded` el mismo commit que `head`?
///
/// `recorded` es la revision que GekkoApp registro al instalar (`git rev-parse
/// --short`, 7 o mas caracteres) y `head` el SHA completo de la rama. Devuelve
/// `None` si alguno no parece un hash (por ejemplo el marcador "instalado"),
/// para no afirmar nada que no se puede comprobar.
pub fn same_commit(recorded: &str, head: &str) -> Option<bool> {
    let is_hash = |value: &str, min: usize| {
        (min..=40).contains(&value.len()) && value.chars().all(|c| c.is_ascii_hexdigit())
    };
    let recorded = recorded.trim().to_ascii_lowercase();
    let head = head.trim().to_ascii_lowercase();
    if !is_hash(&recorded, 7) || !is_hash(&head, 40) {
        return None;
    }
    Some(head.starts_with(&recorded))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "4c3f9bf6ca8d836619226c8e8200da783555d7cd";

    #[test]
    fn same_commit_accepts_the_short_revision_recorded_at_install() {
        assert_eq!(same_commit("4c3f9bf", HEAD), Some(true));
        assert_eq!(same_commit("4C3F9BF6ca", HEAD), Some(true));
        assert_eq!(same_commit(HEAD, HEAD), Some(true));
    }

    #[test]
    fn same_commit_detects_a_newer_branch_head() {
        assert_eq!(same_commit("795828f", HEAD), Some(false));
    }

    #[test]
    fn same_commit_refuses_to_guess_without_a_hash() {
        assert_eq!(same_commit("instalado", HEAD), None);
        assert_eq!(same_commit("2.1.0", HEAD), None);
        assert_eq!(same_commit("4c3f9b", HEAD), None);
        assert_eq!(same_commit("4c3f9bf", "4c3f9bf"), None);
        assert_eq!(same_commit("", HEAD), None);
    }
}
