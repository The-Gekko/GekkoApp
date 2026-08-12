use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentId {
    Compositor,
    Kiui,
    Kitowall,
    Kilivepaper,
    Kisddm,
}

impl ComponentId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Compositor => "Kitsune Compositor",
            Self::Kiui => "KiUI",
            Self::Kitowall => "Kitowall (wallpapers estaticos)",
            Self::Kilivepaper => "Kilivepaper (live wallpapers)",
            Self::Kisddm => "KiSDDM (pantalla de inicio SDDM)",
        }
    }

    pub fn product_id(self) -> &'static str {
        match self {
            Self::Compositor => "kitsune-compositor",
            Self::Kiui => "kiui",
            Self::Kitowall => "kitowall",
            Self::Kilivepaper => "kilivepaper",
            Self::Kisddm => "kisddm",
        }
    }

    pub fn repository(self) -> &'static str {
        match self {
            Self::Compositor => "KitotsuMolina/Kito-compositor",
            Self::Kiui => "KitotsuMolina/KiUI",
            Self::Kitowall => "KitotsuMolina/KitowallV2",
            Self::Kilivepaper => "KitotsuMolina/Kilivepaper",
            Self::Kisddm => "KitotsuMolina/KiSDDM",
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ModuleSelection {
    pub kitowall: bool,
    pub kilivepaper: bool,
    pub kisddm: bool,
}

impl ModuleSelection {
    pub fn has_product(&self) -> bool {
        self.kitowall || self.kilivepaper || self.kisddm
    }

    pub fn plan(&self) -> Vec<ComponentId> {
        let mut components = BTreeSet::from([ComponentId::Compositor, ComponentId::Kiui]);
        if self.kitowall {
            components.insert(ComponentId::Kitowall);
        }
        if self.kilivepaper {
            components.insert(ComponentId::Kilivepaper);
        }
        if self.kisddm {
            components.insert(ComponentId::Kisddm);
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

pub fn resolve_releases(components: &[ComponentId], target: &str) -> Vec<ReleaseStatus> {
    components
        .iter()
        .copied()
        .map(|component| resolve_release(component, target))
        .collect()
}

fn resolve_release(component: ComponentId, target: &str) -> ReleaseStatus {
    let state = match crate::core::github::resolve_latest_release(component.repository(), target) {
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
        assert!(!plan.contains(&ComponentId::Kisddm));
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

    #[test]
    fn kisddm_can_be_selected_as_a_product_module() {
        let selection = ModuleSelection {
            kisddm: true,
            ..ModuleSelection::default()
        };
        assert!(selection.has_product());
        assert!(selection.plan().contains(&ComponentId::Kisddm));
    }
}
