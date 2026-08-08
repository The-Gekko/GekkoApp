use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentId {
    Compositor,
    Kiui,
    Kitowall,
    Kilivepaper,
}

impl ComponentId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Compositor => "Kitsune Compositor",
            Self::Kiui => "KiUI",
            Self::Kitowall => "Kitowall (wallpapers estaticos)",
            Self::Kilivepaper => "Kilivepaper (live wallpapers)",
        }
    }

    pub fn repository(self) -> &'static str {
        match self {
            Self::Compositor => "KitotsuMolina/Kito-compositor",
            Self::Kiui => "KitotsuMolina/KiUI",
            Self::Kitowall => "KitotsuMolina/KitowallV2",
            Self::Kilivepaper => "KitotsuMolina/Kilivepaper",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModuleSelection {
    pub kitowall: bool,
    pub kilivepaper: bool,
}

impl ModuleSelection {
    pub fn has_product(&self) -> bool {
        self.kitowall || self.kilivepaper
    }

    pub fn plan(&self) -> Vec<ComponentId> {
        let mut components = BTreeSet::from([ComponentId::Compositor, ComponentId::Kiui]);
        if self.kitowall {
            components.insert(ComponentId::Kitowall);
        }
        if self.kilivepaper {
            components.insert(ComponentId::Kilivepaper);
        }
        components.into_iter().collect()
    }
}

#[derive(Debug, Clone)]
pub struct ReleaseStatus {
    pub component: ComponentId,
    pub state: ReleaseState,
}

#[derive(Debug, Clone)]
pub enum ReleaseState {
    Available {
        version: String,
        tag: String,
        manifest_url: String,
        asset_urls: BTreeMap<String, String>,
    },
    Unavailable(String),
}

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

pub fn resolve_releases(components: &[ComponentId], target: &str) -> Vec<ReleaseStatus> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_recv_body(Some(Duration::from_secs(15)))
        .user_agent("GekkoApp/1.1")
        .build()
        .into();

    components
        .iter()
        .copied()
        .map(|component| resolve_release(&agent, component, target))
        .collect()
}

fn resolve_release(agent: &ureq::Agent, component: ComponentId, target: &str) -> ReleaseStatus {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        component.repository()
    );
    let result = (|| {
        let mut response = agent
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|error| format!("release no disponible: {error}"))?;
        let body = response
            .body_mut()
            .with_config()
            .limit(2 * 1024 * 1024)
            .read_to_string()
            .map_err(|error| format!("respuesta invalida: {error}"))?;
        let release: GithubRelease =
            serde_json::from_str(&body).map_err(|error| format!("JSON invalido: {error}"))?;
        let suffix = format!("-{target}.manifest.json");
        let manifest = release
            .assets
            .iter()
            .find(|asset| asset.name.ends_with(&suffix))
            .ok_or_else(|| format!("el release {} no incluye {target}", release.tag_name))?;
        let asset_urls = release
            .assets
            .iter()
            .map(|asset| (asset.name.clone(), asset.browser_download_url.clone()))
            .collect();
        Ok::<_, String>((
            release.tag_name,
            manifest.browser_download_url.clone(),
            asset_urls,
        ))
    })();

    let state = match result {
        Ok((tag, manifest_url, asset_urls)) => ReleaseState::Available {
            version: tag.trim_start_matches('v').to_string(),
            tag,
            manifest_url,
            asset_urls,
        },
        Err(reason) => ReleaseState::Unavailable(reason),
    };
    ReleaseStatus { component, state }
}

pub fn all_available(statuses: &[ReleaseStatus]) -> bool {
    statuses
        .iter()
        .all(|status| matches!(status.state, ReleaseState::Available { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mandatory_components_are_always_included() {
        let plan = ModuleSelection {
            kilivepaper: true,
            ..ModuleSelection::default()
        }
        .plan();

        assert!(plan.contains(&ComponentId::Compositor));
        assert!(plan.contains(&ComponentId::Kiui));
        assert!(plan.contains(&ComponentId::Kilivepaper));
        assert!(!plan.contains(&ComponentId::Kitowall));
    }

    #[test]
    fn selection_requires_at_least_one_product_module() {
        assert!(!ModuleSelection::default().has_product());
        assert!(ModuleSelection {
            kitowall: true,
            ..ModuleSelection::default()
        }
        .has_product());
    }
}
