use crate::core::github;
use crate::installer::{
    ensure_https, ArtifactManifest, ComponentIdentity, InstallationPlan, PreparedRelease,
};
use crate::kito::ComponentId;

pub const BAUH_PRODUCT_ID: &str = "bauh-fork-the-gekko";
pub const BAUH_REPOSITORY: &str = "The-Gekko/Bauh-Fork-The-Gekko";
pub const BAUH_LABEL: &str = "Bauh Fork (The-Gekko)";

pub const GEKKO_ADB_PRODUCT_ID: &str = "gekko-adb";
pub const GEKKO_ADB_REPOSITORY: &str = "The-Gekko/gekko-adb";
pub const GEKKO_ADB_LABEL: &str = "Gekko ADB Studio";

pub const GEKKOAPP_PRODUCT_ID: &str = "gekkoapp";
pub const GEKKOAPP_REPOSITORY: &str = "The-Gekko/GekkoApp";
pub const GEKKOAPP_LABEL: &str = "GekkoApp (Control Center)";

/// Identidad de un componente gestionable desde el catalogo de GekkoApp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogComponent {
    Kito(ComponentId),
    BauhFork,
    GekkoAdb,
    GekkoApp,
}

impl CatalogComponent {
    pub fn id(self) -> &'static str {
        match self {
            Self::Kito(component) => component.product_id(),
            Self::BauhFork => BAUH_PRODUCT_ID,
            Self::GekkoAdb => GEKKO_ADB_PRODUCT_ID,
            Self::GekkoApp => GEKKOAPP_PRODUCT_ID,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Kito(component) => component.label(),
            Self::BauhFork => BAUH_LABEL,
            Self::GekkoAdb => GEKKO_ADB_LABEL,
            Self::GekkoApp => GEKKOAPP_LABEL,
        }
    }

    pub fn repository(self) -> &'static str {
        match self {
            Self::Kito(component) => component.repository(),
            Self::BauhFork => BAUH_REPOSITORY,
            Self::GekkoAdb => GEKKO_ADB_REPOSITORY,
            Self::GekkoApp => GEKKOAPP_REPOSITORY,
        }
    }
}

/// Todos los componentes del catalogo, incluyendo los obligatorios de Kito.
pub fn all_components() -> Vec<CatalogComponent> {
    [
        CatalogComponent::Kito(ComponentId::Compositor),
        CatalogComponent::Kito(ComponentId::Kiui),
        CatalogComponent::Kito(ComponentId::Kitowall),
        CatalogComponent::Kito(ComponentId::Kilivepaper),
        CatalogComponent::Kito(ComponentId::Kisddm),
        CatalogComponent::BauhFork,
        CatalogComponent::GekkoAdb,
        CatalogComponent::GekkoApp,
    ]
    .to_vec()
}

/// Resuelve, valida y prepara el plan de instalacion pipx del Bauh Fork.
///
/// El release debe declarar `install_method: "python_pipx"`: el artefacto es
/// un archivo fuente que se instala con `pipx install --force` tras verificar
/// su SHA-256 contra el manifesto.
pub fn resolve_bauh_plan(target: &str) -> Result<InstallationPlan, String> {
    let (tag, manifest_url, asset_urls) = github::resolve_latest_release(BAUH_REPOSITORY, target)?;
    ensure_https(&manifest_url)?;
    let bytes = github::download_manifest_body(&manifest_url)?;
    let manifest: ArtifactManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("manifiesto de {BAUH_LABEL} invalido: {error}"))?;
    let artifact_url = asset_urls
        .get(&manifest.artifact.file_name)
        .ok_or_else(|| {
            format!(
                "el release de {BAUH_LABEL} no incluye el artefacto {}",
                manifest.artifact.file_name
            )
        })?;
    let release = PreparedRelease::prepare_pipx(
        ComponentIdentity {
            label: BAUH_LABEL,
            product_id: BAUH_PRODUCT_ID,
            repository: BAUH_REPOSITORY,
        },
        &tag,
        target,
        &manifest_url,
        artifact_url,
        manifest,
    )?;
    Ok(InstallationPlan::single(release))
}

/// Resuelve, valida y prepara el plan de auto-actualizacion de GekkoApp.
///
/// Usa el mismo motor que el Bauh Fork, pero con `install_method:
/// "binary_extract"`: el artefacto es un tarball con los binarios y la
/// integracion de escritorio que se activa con el layout nativo de symlinks.
pub fn resolve_gekkoapp_plan(target: &str) -> Result<InstallationPlan, String> {
    let (tag, manifest_url, asset_urls) =
        github::resolve_latest_release(GEKKOAPP_REPOSITORY, target)?;
    ensure_https(&manifest_url)?;
    let bytes = github::download_manifest_body(&manifest_url)?;
    let manifest: ArtifactManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("manifiesto de {GEKKOAPP_LABEL} invalido: {error}"))?;
    let artifact_url = asset_urls
        .get(&manifest.artifact.file_name)
        .ok_or_else(|| {
            format!(
                "el release de {GEKKOAPP_LABEL} no incluye el artefacto {}",
                manifest.artifact.file_name
            )
        })?;
    let release = PreparedRelease::prepare_native(
        ComponentIdentity {
            label: GEKKOAPP_LABEL,
            product_id: GEKKOAPP_PRODUCT_ID,
            repository: GEKKOAPP_REPOSITORY,
        },
        &tag,
        target,
        &manifest_url,
        artifact_url,
        manifest,
    )?;
    Ok(InstallationPlan::single(release))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn resolves_published_bauh_release_from_github() {
        let plan = resolve_bauh_plan("x86_64-unknown-linux-gnu").expect("release resoluble");
        assert_eq!(plan.releases.len(), 1);
        let release = &plan.releases[0];
        assert_eq!(release.component_label, BAUH_LABEL);
        assert_eq!(release.manifest.product.version, "0.10.7");
        assert_eq!(release.manifest.install_method, "python_pipx");
        assert!(release
            .manifest_url
            .starts_with("https://github.com/The-Gekko/Bauh-Fork-The-Gekko/"));
    }
}
