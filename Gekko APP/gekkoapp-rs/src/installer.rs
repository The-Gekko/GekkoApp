use crate::kito::{ReleaseState, ReleaseStatus};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MANIFEST_LIMIT: u64 = 2 * 1024 * 1024;
const ARTIFACT_LIMIT: u64 = 512 * 1024 * 1024;
const STATE_SCHEMA_VERSION: u32 = 1;

pub(crate) const MANIFEST_LIMIT_BYTES: u64 = MANIFEST_LIMIT;

fn default_install_method() -> String {
    "native".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArtifactManifest {
    pub schema_version: u32,
    pub kind: String,
    pub distribution_contract: String,
    #[serde(default = "default_install_method")]
    pub install_method: String,
    /// Nombre de la distribucion Python con la que pipx registra el paquete.
    ///
    /// Solo aplica a `install_method: "python_pipx"`. pipx crea y desinstala el
    /// entorno por el `[project] name` de `pyproject.toml`, que no tiene por que
    /// coincidir con el id de producto ni con el del ejecutable. Si el
    /// manifiesto no lo declara se deduce del arbol ya verificado.
    #[serde(default)]
    pub pipx_distribution: String,
    pub product: Product,
    pub release: Release,
    pub platform: Platform,
    pub artifact: Artifact,
    pub payload: Vec<PayloadEntry>,
    pub entrypoints: Vec<Entrypoint>,
    pub requirements: Requirements,
    pub integrations: Integrations,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Product {
    pub id: String,
    pub version: String,
    pub repository: String,
    #[serde(default)]
    pub contract_version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag: String,
    pub channel: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Platform {
    pub os: String,
    pub arch: String,
    pub target: String,
    pub libc: Libc,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Libc {
    pub family: String,
    pub minimum: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    pub file_name: String,
    pub format: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PayloadEntry {
    pub path: String,
    pub kind: String,
    pub mode: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Entrypoint {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Requirements {
    #[serde(default)]
    pub modules: Vec<ModuleRequirement>,
    #[serde(default)]
    pub host_capabilities: Vec<HostCapability>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModuleRequirement {
    pub id: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HostCapability {
    pub id: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Integrations {
    #[serde(default)]
    pub desktop_entries: Vec<DesktopEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DesktopEntry {
    pub application_id: String,
    pub template: String,
    pub entrypoint: String,
    pub icons: Vec<DesktopIcon>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DesktopIcon {
    pub source: String,
    pub theme: String,
    pub size: u32,
    pub format: String,
}

#[derive(Debug, Clone)]
pub struct PreparedRelease {
    pub component_label: String,
    pub manifest_url: String,
    pub artifact_url: String,
    pub manifest: ArtifactManifest,
}

/// Identidad verificable de un componente instalado por el motor.
pub struct ComponentIdentity<'a> {
    pub label: &'a str,
    pub product_id: &'a str,
    pub repository: &'a str,
}

impl PreparedRelease {
    /// Build a pipx-managed release after validating the manifest identity.
    ///
    /// Used by catalog components that are not Kito products (e.g. Bauh Fork),
    /// which install via `pipx` instead of the native symlink layout.
    pub fn prepare_pipx(
        identity: ComponentIdentity<'_>,
        tag: &str,
        target: &str,
        manifest_url: &str,
        artifact_url: &str,
        manifest: ArtifactManifest,
    ) -> Result<Self, String> {
        if manifest.install_method != "python_pipx" {
            return Err(format!("{} no usa el metodo python_pipx", identity.label));
        }
        Self::prepare_verified(identity, tag, target, manifest_url, artifact_url, manifest)
    }

    /// Build a native symlink-layout release after validating the manifest
    /// identity (e.g. the GekkoApp self-update with `binary_extract`).
    pub fn prepare_native(
        identity: ComponentIdentity<'_>,
        tag: &str,
        target: &str,
        manifest_url: &str,
        artifact_url: &str,
        manifest: ArtifactManifest,
    ) -> Result<Self, String> {
        if manifest.install_method != "binary_extract" {
            return Err(format!(
                "{} no usa el metodo binary_extract",
                identity.label
            ));
        }
        Self::prepare_verified(identity, tag, target, manifest_url, artifact_url, manifest)
    }

    fn prepare_verified(
        identity: ComponentIdentity<'_>,
        tag: &str,
        target: &str,
        manifest_url: &str,
        artifact_url: &str,
        manifest: ArtifactManifest,
    ) -> Result<Self, String> {
        validate_manifest(
            &manifest,
            identity.label,
            identity.product_id,
            identity.repository,
            tag,
            target,
        )?;
        let host_glibc = detect_glibc_version()?;
        let minimum_glibc = manifest
            .platform
            .libc
            .minimum
            .as_deref()
            .ok_or_else(|| format!("{} no declara glibc minima", identity.label))?;
        if compare_versions(&host_glibc, minimum_glibc)? == std::cmp::Ordering::Less {
            return Err(format!(
                "{} requiere glibc {minimum_glibc}, pero el sistema tiene {host_glibc}",
                identity.label
            ));
        }
        ensure_https(manifest_url)?;
        ensure_https(artifact_url)?;
        Ok(Self {
            component_label: identity.label.to_string(),
            manifest_url: manifest_url.to_string(),
            artifact_url: artifact_url.to_string(),
            manifest,
        })
    }
}

#[derive(Debug, Clone)]
pub struct InstallationPlan {
    pub releases: Vec<PreparedRelease>,
}

#[derive(Debug, Clone)]
pub struct InstallPaths {
    pub bin_home: PathBuf,
    pub data_home: PathBuf,
    pub state_home: PathBuf,
    pub cache_home: PathBuf,
    pub versions_home: PathBuf,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct InstallationState {
    pub schema_version: u32,
    #[serde(default)]
    pub modules: BTreeMap<String, InstalledModule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstalledModule {
    pub version: String,
    pub contract_version: String,
    pub active_root: String,
    #[serde(default)]
    pub manifest_url: String,
    pub artifact_url: String,
    pub artifact_sha256: String,
    pub entrypoints: BTreeMap<String, String>,
    pub owned_paths: Vec<OwnedPath>,
    pub activated_at_unix: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OwnedPath {
    pub path: String,
    pub kind: String,
    pub sha256: Option<String>,
    pub target: Option<String>,
}

impl InstallPaths {
    pub fn detect() -> Result<Self, String> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME no esta definido".to_owned())?;
        let data_home = env_path("XDG_DATA_HOME").unwrap_or_else(|| home.join(".local/share"));
        let state_home = env_path("XDG_STATE_HOME").unwrap_or_else(|| home.join(".local/state"));
        let cache_home = env_path("XDG_CACHE_HOME").unwrap_or_else(|| home.join(".cache"));
        let bin_home = env_path("XDG_BIN_HOME").unwrap_or_else(|| home.join(".local/bin"));
        Ok(Self {
            bin_home,
            data_home,
            state_home: state_home.join("gekkoapp"),
            cache_home: cache_home.join("gekkoapp"),
            versions_home: home.join(".local/lib/kitotsu"),
        })
    }

    pub fn state_file(&self) -> PathBuf {
        self.state_home.join("installations-v1.json")
    }
}

impl InstallationPlan {
    pub fn prepare(statuses: &[ReleaseStatus], target: &str) -> Result<Self, String> {
        let agent = http_agent();
        let host_glibc = detect_glibc_version()?;
        let mut releases = Vec::with_capacity(statuses.len());
        for status in statuses {
            let ReleaseState::Available {
                tag,
                manifest_url,
                asset_urls,
                ..
            } = &status.state
            else {
                return Err(format!(
                    "{} no tiene un release disponible",
                    status.component.label()
                ));
            };
            ensure_https(manifest_url)?;
            let bytes = download_bytes(&agent, manifest_url, MANIFEST_LIMIT)?;
            let manifest: ArtifactManifest = serde_json::from_slice(&bytes).map_err(|error| {
                format!(
                    "manifiesto de {} invalido: {error}",
                    status.component.label()
                )
            })?;
            validate_manifest(
                &manifest,
                status.component.label(),
                status.component.product_id(),
                status.component.repository(),
                tag,
                target,
            )?;
            // El metodo de activacion NO puede venir elegido por el manifiesto
            // remoto: los componentes del catalogo Kito se activan siempre con
            // el layout nativo de symlinks. Sin este corte, un release
            // manipulado podria declarar `python_pipx` y conseguir que GekkoApp
            // ejecute `pipx install` (y por tanto codigo de build arbitrario)
            // sobre el arbol recien extraido.
            if !matches!(
                manifest.install_method.as_str(),
                "native" | "binary_extract"
            ) {
                return Err(format!(
                    "{} declara un metodo de instalacion no permitido: {}",
                    status.component.label(),
                    manifest.install_method
                ));
            }
            let minimum_glibc =
                manifest.platform.libc.minimum.as_deref().ok_or_else(|| {
                    format!("{} no declara glibc minima", status.component.label())
                })?;
            if compare_versions(&host_glibc, minimum_glibc)? == std::cmp::Ordering::Less {
                return Err(format!(
                    "{} requiere glibc {minimum_glibc}, pero el sistema tiene {host_glibc}",
                    status.component.label()
                ));
            }
            let artifact_url = asset_urls
                .get(&manifest.artifact.file_name)
                .ok_or_else(|| {
                    format!(
                        "el release de {} no contiene {}",
                        status.component.label(),
                        manifest.artifact.file_name
                    )
                })?
                .clone();
            ensure_https(&artifact_url)?;
            releases.push(PreparedRelease {
                component_label: status.component.label().to_string(),
                manifest_url: manifest_url.clone(),
                artifact_url,
                manifest,
            });
        }
        validate_module_dependencies(&releases)?;
        Ok(Self { releases })
    }

    /// Convenience plan for a single managed component (e.g. Bauh Fork o el
    /// propio GekkoApp). La activacion se elige por `install_method` del
    /// manifiesto (`python_pipx` vs layout nativo de symlinks).
    pub fn single(release: PreparedRelease) -> Self {
        Self {
            releases: vec![release],
        }
    }

    /// Paquetes de host que exigen las capacidades obligatorias del plan.
    ///
    /// Devuelve `(paquetes, capacidades_sin_soporte)`. Los nombres dependen de
    /// la distribucion: el mapeo anterior era solo de Arch y en Solus mandaba a
    /// eopkg nombres inexistentes (`vulkan-icd-loader`, `awww`), asi que la
    /// instalacion fallaba a mitad. Las capacidades sin equivalente se devuelven
    /// aparte para poder abortar antes de tocar el sistema.
    pub fn required_host_packages(&self, solus: bool) -> (Vec<&'static str>, Vec<&'static str>) {
        let mut packages = BTreeSet::new();
        let mut unsupported = BTreeSet::new();
        for release in &self.releases {
            for capability in &release.manifest.requirements.host_capabilities {
                if capability.optional {
                    continue;
                }
                match (capability.id.as_str(), solus) {
                    ("runtime.qt6", _) => {
                        packages.extend(["qt6-base", "qt6-declarative", "qt6-wayland"]);
                    }
                    ("renderer.awww", false) => {
                        packages.insert("awww");
                    }
                    ("renderer.awww", true) => {
                        unsupported.insert("renderer.awww");
                    }
                    ("gpu.wgpu", false) => {
                        packages.extend(["vulkan-icd-loader", "wayland", "libxkbcommon"]);
                    }
                    ("gpu.wgpu", true) => {
                        packages.extend(["vulkan", "wayland", "libxkbcommon"]);
                    }
                    ("audio.pipewire", _) => {
                        packages.insert("pipewire");
                    }
                    _ => {}
                }
            }
        }
        (
            packages.into_iter().collect(),
            unsupported.into_iter().collect(),
        )
    }

    pub fn prefetch(&self, paths: &InstallPaths) -> Result<(), String> {
        fs::create_dir_all(&paths.cache_home).map_err(io_error("crear cache"))?;
        for release in &self.releases {
            download_artifact(release, paths)?;
        }
        Ok(())
    }

    pub fn install(&self, paths: &InstallPaths) -> Result<InstallationState, String> {
        fs::create_dir_all(&paths.cache_home).map_err(io_error("crear cache"))?;
        fs::create_dir_all(&paths.versions_home).map_err(io_error("crear raiz de versiones"))?;
        fs::create_dir_all(&paths.bin_home).map_err(io_error("crear directorio bin"))?;
        fs::create_dir_all(&paths.state_home).map_err(io_error("crear directorio de estado"))?;

        let mut state = load_state(&paths.state_file())?;
        let mut installed = Vec::with_capacity(self.releases.len());
        for release in &self.releases {
            let archive = download_artifact(release, paths)?;
            let root = install_version(release, &archive, paths)?;
            installed.push((release, root));
        }
        // Una instalacion pipx previa del mismo paquete (hecha por el propio
        // instalador del proyecto, por ejemplo) deja sus lanzadores en
        // `bin_home`. Se retira antes de validar destinos: si no, la validacion
        // los tomaria por "rutas ajenas" y abortaria toda la actualizacion.
        for (release, root) in &installed {
            if release.manifest.install_method == "python_pipx" {
                remove_previous_pipx_installation(release, root);
            }
        }
        validate_activation_destinations(&installed, paths, &state)?;
        for (release, root) in installed {
            let activation = if release.manifest.install_method == "python_pipx" {
                activate_pipx_release(release, &root, paths, &mut state)
            } else {
                activate_release(release, &root, paths, &mut state)
            };
            // El estado se escribe tras cada activacion. Si una falla a mitad de
            // un plan de varios componentes, lo ya activado queda registrado:
            // se puede desinstalar y un reintento no lo vera como "ruta ajena".
            // El error de activacion tiene prioridad sobre el de escritura: es
            // el que explica de verdad que ha pasado.
            let persisted = write_state(&paths.state_file(), &state);
            activation?;
            persisted?;
        }
        Ok(state)
    }
}

fn validate_activation_destinations(
    installed: &[(&PreparedRelease, PathBuf)],
    paths: &InstallPaths,
    state: &InstallationState,
) -> Result<(), String> {
    for (release, _) in installed {
        let owned = state
            .modules
            .get(&release.manifest.product.id)
            .map(|module| {
                module
                    .owned_paths
                    .iter()
                    .map(|path| path.path.as_str())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        for entrypoint in &release.manifest.entrypoints {
            ensure_destination_available(&paths.bin_home.join(&entrypoint.name), &owned)?;
        }
        for desktop in &release.manifest.integrations.desktop_entries {
            ensure_destination_available(
                &paths
                    .data_home
                    .join("applications")
                    .join(format!("{}.desktop", desktop.application_id)),
                &owned,
            )?;
            for icon in &desktop.icons {
                ensure_destination_available(
                    &paths
                        .data_home
                        .join("icons")
                        .join(&icon.theme)
                        .join(format!("{}x{}", icon.size, icon.size))
                        .join("apps")
                        .join(format!("{}.{}", desktop.application_id, icon.format)),
                    &owned,
                )?;
            }
        }
    }
    Ok(())
}

fn ensure_destination_available(path: &Path, owned: &BTreeSet<&str>) -> Result<(), String> {
    if fs::symlink_metadata(path).is_ok() && !owned.contains(path_text(path)?.as_str()) {
        return Err(format!(
            "la activacion pisaria una ruta ajena: {}",
            path.display()
        ));
    }
    Ok(())
}

/// Adopta las rutas que un plan va a escribir y que todavia no estan
/// registradas como propias en el estado (p. ej. los binarios e integracion
/// previos de GekkoApp instalados con `scripts/install.sh`).
///
/// Solo debe usarse en el flujo de auto-actualizacion del propio GekkoApp,
/// que deliberadamente reclama sus rutas legacy; nunca en componentes de
/// terceros.
pub(crate) fn adopt_release_destinations(
    paths: &InstallPaths,
    plan: &InstallationPlan,
) -> Result<(), String> {
    let state = load_state(&paths.state_file())?;
    for release in &plan.releases {
        let owned = state
            .modules
            .get(&release.manifest.product.id)
            .map(|module| {
                module
                    .owned_paths
                    .iter()
                    .map(|path| path.path.as_str())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let mut destinations = Vec::new();
        for entrypoint in &release.manifest.entrypoints {
            destinations.push(paths.bin_home.join(&entrypoint.name));
        }
        for desktop in &release.manifest.integrations.desktop_entries {
            destinations.push(
                paths
                    .data_home
                    .join("applications")
                    .join(format!("{}.desktop", desktop.application_id)),
            );
            for icon in &desktop.icons {
                destinations.push(
                    paths
                        .data_home
                        .join("icons")
                        .join(&icon.theme)
                        .join(format!("{}x{}", icon.size, icon.size))
                        .join("apps")
                        .join(format!("{}.{}", desktop.application_id, icon.format)),
                );
            }
        }
        for destination in destinations {
            let text = path_text(&destination)?;
            if fs::symlink_metadata(&destination).is_ok() && !owned.contains(text.as_str()) {
                let _ = fs::remove_file(&destination);
            }
        }
    }
    Ok(())
}

pub(crate) fn http_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(180)))
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_body(Some(Duration::from_secs(120)))
        .user_agent("GekkoApp/1.1")
        .build()
        .into()
}

pub(crate) fn download_bytes(
    agent: &ureq::Agent,
    url: &str,
    limit: u64,
) -> Result<Vec<u8>, String> {
    let mut response = agent
        .get(url)
        .header("Accept", "application/octet-stream")
        .call()
        .map_err(|error| format!("no se pudo descargar {url}: {error}"))?;
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|error| format!("respuesta de {url} invalida: {error}"))
}

fn download_artifact(release: &PreparedRelease, paths: &InstallPaths) -> Result<PathBuf, String> {
    let artifacts = paths.cache_home.join("artifacts");
    fs::create_dir_all(&artifacts).map_err(io_error("crear cache de artefactos"))?;
    let destination = artifacts.join(&release.manifest.artifact.file_name);
    if destination.is_file()
        && file_size(&destination)? == release.manifest.artifact.size_bytes
        && sha256_file(&destination)?.eq_ignore_ascii_case(&release.manifest.artifact.sha256)
    {
        return Ok(destination);
    }

    let temporary = artifacts.join(format!(
        ".{}.download-{}",
        release.manifest.artifact.file_name,
        std::process::id()
    ));
    let _ = fs::remove_file(&temporary);
    let agent = http_agent();
    let mut response = agent
        .get(&release.artifact_url)
        .header("Accept", "application/octet-stream")
        .call()
        .map_err(|error| format!("no se pudo descargar {}: {error}", release.artifact_url))?;
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(ARTIFACT_LIMIT)
        .reader();
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(io_error("crear descarga temporal"))?;
    // Cualquier salida por error a partir de aqui debe borrar el temporal: si
    // no, una descarga interrumpida deja basura en la cache para siempre.
    let cleanup = |error: String| -> String {
        let _ = fs::remove_file(&temporary);
        error
    };
    io::copy(&mut reader, &mut output)
        .map_err(io_error("descargar artefacto"))
        .map_err(cleanup)?;
    output
        .sync_all()
        .map_err(io_error("sincronizar artefacto"))
        .map_err(cleanup)?;

    let size = file_size(&temporary).map_err(cleanup)?;
    let digest = sha256_file(&temporary).map_err(cleanup)?;
    if size != release.manifest.artifact.size_bytes
        || !digest.eq_ignore_ascii_case(&release.manifest.artifact.sha256)
    {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "el artefacto {} no coincide con su manifiesto",
            release.manifest.artifact.file_name
        ));
    }
    fs::rename(&temporary, &destination)
        .map_err(io_error("activar artefacto en cache"))
        .map_err(cleanup)?;
    Ok(destination)
}

fn install_version(
    release: &PreparedRelease,
    archive_path: &Path,
    paths: &InstallPaths,
) -> Result<PathBuf, String> {
    let product = &release.manifest.product.id;
    let version = &release.manifest.product.version;
    let product_home = paths.versions_home.join(product);
    let final_root = product_home.join(version);
    if final_root.is_dir() {
        verify_payload(&final_root, &release.manifest.payload)?;
        return Ok(final_root);
    }
    fs::create_dir_all(&product_home).map_err(io_error("crear directorio del producto"))?;
    let staging = product_home.join(format!(".staging-{version}-{}", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(io_error("limpiar staging anterior"))?;
    }
    fs::create_dir_all(&staging).map_err(io_error("crear staging"))?;

    let result = extract_archive(archive_path, &staging, &release.manifest);
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let package_root = staging.join(format!("{product}-{version}"));
    // El staging se retira tambien cuando la verificacion o la activacion
    // fallan; antes solo se limpiaba en el camino feliz y en el de extraccion.
    let discard_staging = |error: String| -> String {
        let _ = fs::remove_dir_all(&staging);
        error
    };
    verify_payload(&package_root, &release.manifest.payload).map_err(discard_staging)?;
    fs::rename(&package_root, &final_root)
        .map_err(io_error("activar version instalada"))
        .map_err(discard_staging)?;
    fs::remove_dir_all(&staging).map_err(io_error("retirar staging"))?;
    Ok(final_root)
}

fn extract_archive(
    archive_path: &Path,
    staging: &Path,
    manifest: &ArtifactManifest,
) -> Result<(), String> {
    let file = File::open(archive_path).map_err(io_error("abrir artefacto"))?;
    let decoder = zstd::stream::read::Decoder::new(file)
        .map_err(|error| format!("zstd invalido: {error}"))?;
    let mut archive = tar::Archive::new(decoder);
    let expected_root = format!("{}-{}", manifest.product.id, manifest.product.version);
    let expected_files = manifest
        .payload
        .iter()
        .map(|entry| format!("{expected_root}/{}", entry.path))
        .collect::<BTreeSet<_>>();
    // Tamano declarado por archivo: se comprueba contra la cabecera tar ANTES
    // de escribir nada. El limite de descarga solo acota el .tar.zst
    // comprimido, asi que sin esto una bomba de descompresion podria llenar el
    // disco antes de que `verify_payload` llegase a mirar los hashes.
    let expected_sizes = manifest
        .payload
        .iter()
        .map(|entry| (format!("{expected_root}/{}", entry.path), entry.size_bytes))
        .collect::<BTreeMap<_, _>>();
    let mut found_files = BTreeSet::new();

    for entry in archive
        .entries()
        .map_err(|error| format!("tar invalido: {error}"))?
    {
        let mut entry = entry.map_err(|error| format!("entrada tar invalida: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("ruta tar invalida: {error}"))?
            .into_owned();
        validate_relative_path(&path)?;
        if path.components().next().and_then(component_text) != Some(expected_root.as_str()) {
            return Err(format!(
                "raiz inesperada dentro del artefacto: {}",
                path.display()
            ));
        }
        let entry_type = entry.header().entry_type();
        if entry_type.is_file() {
            let text = path_to_manifest_string(&path)?;
            if !expected_files.contains(&text) {
                return Err(format!("archivo no declarado en payload: {text}"));
            }
            let declared = expected_sizes.get(&text).copied().unwrap_or_default();
            if entry.header().size().unwrap_or(u64::MAX) != declared {
                return Err(format!(
                    "el tamano de {text} dentro del artefacto no coincide con el payload"
                ));
            }
            found_files.insert(text);
        } else if !entry_type.is_dir() {
            return Err(format!(
                "tipo de entrada tar no permitido: {}",
                path.display()
            ));
        }
        entry
            .unpack_in(staging)
            .map_err(|error| format!("no se pudo extraer {}: {error}", path.display()))?;
    }
    if found_files != expected_files {
        return Err("el contenido del artefacto no coincide con payload".into());
    }
    Ok(())
}

fn verify_payload(root: &Path, payload: &[PayloadEntry]) -> Result<(), String> {
    for entry in payload {
        validate_manifest_path(&entry.path)?;
        let path = root.join(&entry.path);
        let metadata = fs::symlink_metadata(&path).map_err(io_error("leer payload"))?;
        if !metadata.file_type().is_file() {
            return Err(format!("payload no es un archivo regular: {}", entry.path));
        }
        if metadata.len() != entry.size_bytes
            || !sha256_file(&path)?.eq_ignore_ascii_case(&entry.sha256)
        {
            return Err(format!("hash o tamano invalido en payload: {}", entry.path));
        }
        let expected_mode = u32::from_str_radix(entry.mode.trim_start_matches('0'), 8)
            .map_err(|_| format!("modo invalido en payload: {}", entry.mode))?;
        if metadata.permissions().mode() & 0o777 != expected_mode {
            return Err(format!("modo invalido en payload: {}", entry.path));
        }
    }
    Ok(())
}

fn activate_release(
    release: &PreparedRelease,
    root: &Path,
    paths: &InstallPaths,
    state: &mut InstallationState,
) -> Result<(), String> {
    let product = &release.manifest.product.id;
    let previous_owned = state
        .modules
        .get(product)
        .map(|module| {
            module
                .owned_paths
                .iter()
                .map(|owned| owned.path.as_str())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let mut entrypoints = BTreeMap::new();
    let mut owned_paths = Vec::new();

    for entrypoint in &release.manifest.entrypoints {
        validate_entrypoint_name(&entrypoint.name)?;
        validate_manifest_path(&entrypoint.path)?;
        let target = root.join(&entrypoint.path);
        if !target.is_file() {
            return Err(format!("entrypoint ausente: {}", entrypoint.path));
        }
        let link = paths.bin_home.join(&entrypoint.name);
        activate_symlink(
            &target,
            &link,
            previous_owned.contains(path_text(&link)?.as_str()),
        )?;
        entrypoints.insert(entrypoint.name.clone(), link.display().to_string());
        owned_paths.push(OwnedPath {
            path: link.display().to_string(),
            kind: "symlink".into(),
            sha256: None,
            target: Some(target.display().to_string()),
        });
    }

    for desktop in &release.manifest.integrations.desktop_entries {
        activate_desktop_integration(
            desktop,
            root,
            paths,
            &entrypoints,
            &previous_owned,
            &mut owned_paths,
        )?;
    }

    state.schema_version = STATE_SCHEMA_VERSION;
    state.modules.insert(
        product.clone(),
        InstalledModule {
            version: release.manifest.product.version.clone(),
            contract_version: release.manifest.product.contract_version.clone(),
            active_root: root.display().to_string(),
            manifest_url: release.manifest_url.clone(),
            artifact_url: release.artifact_url.clone(),
            artifact_sha256: release.manifest.artifact.sha256.clone(),
            entrypoints,
            owned_paths,
            activated_at_unix: now_unix()?,
        },
    );
    Ok(())
}

/// Activate a pipx-managed release: `pipx install` the verified source tree
/// (extracted root), uninstalling any previous pipx-managed package whose bin
/// launchers would block activation, then materialize desktop integration and
/// record the resulting launchers and files as owned.
fn activate_pipx_release(
    release: &PreparedRelease,
    root: &Path,
    paths: &InstallPaths,
    state: &mut InstallationState,
) -> Result<(), String> {
    let product = &release.manifest.product.id;
    let previous_owned = state
        .modules
        .get(product)
        .map(|module| {
            module
                .owned_paths
                .iter()
                .map(|owned| owned.path.as_str())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    // La instalacion pipx previa ya se retiro en `InstallationPlan::install`.
    // `--force` reinstala aunque pipx considere el paquete presente y le deja
    // sobrescribir los lanzadores que el mismo gestiona.
    let pipx = Command::new("pipx")
        .arg("install")
        .arg("--force")
        .arg(root)
        .status()
        .map_err(|error| format!("no se pudo ejecutar pipx: {error}"))?;
    if !pipx.success() {
        return Err(format!(
            "pipx no pudo instalar el artefacto verificado de {}",
            release.component_label
        ));
    }

    let mut entrypoints = BTreeMap::new();
    let mut owned_paths = Vec::new();
    for entrypoint in &release.manifest.entrypoints {
        validate_entrypoint_name(&entrypoint.name)?;
        let launcher = paths.bin_home.join(&entrypoint.name);
        if !launcher.is_file() {
            return Err(format!(
                "pipx no creo el lanzador esperado: {}. El manifiesto declara el \
                 entrypoint '{}', pero la distribucion instalada no lo publica en \
                 [project.scripts]: revisa que el release y el proyecto declaren los \
                 mismos ejecutables.",
                launcher.display(),
                entrypoint.name
            ));
        }
        entrypoints.insert(entrypoint.name.clone(), launcher.display().to_string());
        owned_paths.push(OwnedPath {
            path: launcher.display().to_string(),
            kind: "file".into(),
            sha256: None,
            target: None,
        });
    }

    for desktop in &release.manifest.integrations.desktop_entries {
        activate_desktop_integration(
            desktop,
            root,
            paths,
            &entrypoints,
            &previous_owned,
            &mut owned_paths,
        )?;
    }

    state.schema_version = STATE_SCHEMA_VERSION;
    state.modules.insert(
        product.clone(),
        InstalledModule {
            version: release.manifest.product.version.clone(),
            contract_version: release.manifest.product.contract_version.clone(),
            active_root: root.display().to_string(),
            manifest_url: release.manifest_url.clone(),
            artifact_url: release.artifact_url.clone(),
            artifact_sha256: release.manifest.artifact.sha256.clone(),
            entrypoints,
            owned_paths,
            activated_at_unix: now_unix()?,
        },
    );
    Ok(())
}

/// Nombre de la distribucion Python con la que pipx registra un release.
///
/// pipx crea y desinstala el entorno por el `[project] name` de
/// `pyproject.toml`, no por el id de producto del release ni por el nombre del
/// ejecutable. Se usa el valor declarado en el manifiesto y, si no lo trae, el
/// que aparece en el arbol ya extraido y verificado por hash.
fn pipx_distribution_name(release: &PreparedRelease, root: &Path) -> Option<String> {
    let declared = release.manifest.pipx_distribution.trim();
    if !declared.is_empty() {
        return valid_distribution_name(declared);
    }
    read_pyproject_name(root).and_then(|name| valid_distribution_name(&name))
}

/// Filtra nombres de distribucion que no cumplen el formato de PEP 508.
///
/// El valor llega de un manifiesto remoto y acaba como argumento de
/// `pipx uninstall`: aunque no pasa por una shell, un nombre que empezara por
/// `-` se interpretaria como una opcion de pipx.
fn valid_distribution_name(name: &str) -> Option<String> {
    let valido = !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte));
    valido.then(|| name.to_string())
}

/// Lee `[project] name` de un `pyproject.toml` sin dependencias de TOML.
fn read_pyproject_name(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root.join("pyproject.toml")).ok()?;
    let mut in_project = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_project = line == "[project]";
            continue;
        }
        if !in_project {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "name" {
            continue;
        }
        let value = value
            .split('#')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches(|character| character == '"' || character == '\'');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

/// Retira, en la medida de lo posible, una instalacion pipx previa del paquete
/// que se va a activar, para que sus lanzadores no bloqueen la validacion de
/// destinos. Los fallos se ignoran a proposito: si no habia nada instalado,
/// `pipx uninstall` termina en error y eso no es un problema.
fn remove_previous_pipx_installation(release: &PreparedRelease, root: &Path) {
    // Se intenta siempre, sin comprobar antes si hay lanzadores en `bin_home`:
    // los ejecutables de la instalacion previa no tienen por que llamarse como
    // los que declara este manifiesto (es justo lo que ocurrio cuando el fork de
    // Bauh renombro sus scripts a `gekko-bauh*`). Si no habia nada instalado,
    // `pipx uninstall` termina en error y se ignora.
    let Some(distribution) = pipx_distribution_name(release, root) else {
        return;
    };
    let _ = Command::new("pipx")
        .args(["uninstall", &distribution])
        .status();
}

fn activate_desktop_integration(
    desktop: &DesktopEntry,
    root: &Path,
    paths: &InstallPaths,
    entrypoints: &BTreeMap<String, String>,
    previous_owned: &BTreeSet<&str>,
    owned_paths: &mut Vec<OwnedPath>,
) -> Result<(), String> {
    validate_application_id(&desktop.application_id)?;
    validate_manifest_path(&desktop.template)?;
    let executable = entrypoints.get(&desktop.entrypoint).ok_or_else(|| {
        format!(
            "entrypoint de escritorio desconocido: {}",
            desktop.entrypoint
        )
    })?;
    let template = fs::read_to_string(root.join(&desktop.template))
        .map_err(io_error("leer plantilla desktop"))?;
    let executable = desktop_exec_quote(executable);
    let mut materialized = template
        .replace("@EXECUTABLE@", &executable)
        .replace("@APPLICATION_ID@", &desktop.application_id);
    if desktop.entrypoint == "kiui" {
        let exec = format!("Exec={} {}", kiui_runtime_environment(paths)?, executable);
        materialized = materialized.replace(&format!("Exec={executable}"), &exec);
    }
    if materialized.contains('@') {
        return Err("la plantilla desktop contiene tokens no admitidos".into());
    }
    let desktop_path = paths
        .data_home
        .join("applications")
        .join(format!("{}.desktop", desktop.application_id));
    write_owned_file(
        &desktop_path,
        materialized.as_bytes(),
        previous_owned.contains(path_text(&desktop_path)?.as_str()),
    )?;
    owned_paths.push(OwnedPath {
        path: desktop_path.display().to_string(),
        kind: "generated".into(),
        sha256: Some(sha256_file(&desktop_path)?),
        target: None,
    });

    for icon in &desktop.icons {
        validate_manifest_path(&icon.source)?;
        if icon.theme != "hicolor" || icon.format != "png" {
            return Err("integracion de icono no soportada".into());
        }
        let source = root.join(&icon.source);
        let destination = paths
            .data_home
            .join("icons")
            .join(&icon.theme)
            .join(format!("{}x{}", icon.size, icon.size))
            .join("apps")
            .join(format!("{}.{}", desktop.application_id, icon.format));
        let bytes = fs::read(&source).map_err(io_error("leer icono"))?;
        write_owned_file(
            &destination,
            &bytes,
            previous_owned.contains(path_text(&destination)?.as_str()),
        )?;
        owned_paths.push(OwnedPath {
            path: destination.display().to_string(),
            kind: "file".into(),
            sha256: Some(sha256_file(&destination)?),
            target: None,
        });
    }
    Ok(())
}

fn activate_symlink(target: &Path, link: &Path, owned: bool) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(link) {
        if !owned || !metadata.file_type().is_symlink() {
            return Err(format!(
                "no se sobreescribira una ruta ajena: {}",
                link.display()
            ));
        }
    }
    let parent = link
        .parent()
        .ok_or_else(|| "entrypoint sin directorio".to_owned())?;
    fs::create_dir_all(parent).map_err(io_error("crear directorio de entrypoints"))?;
    let temporary = parent.join(format!(
        ".{}.gekkoapp-{}",
        link.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("entrypoint"),
        std::process::id()
    ));
    let _ = fs::remove_file(&temporary);
    symlink(target, &temporary).map_err(io_error("crear entrypoint temporal"))?;
    fs::rename(&temporary, link).map_err(io_error("activar entrypoint"))?;
    Ok(())
}

fn write_owned_file(path: &Path, bytes: &[u8], owned: bool) -> Result<(), String> {
    if path.exists() && !owned {
        return Err(format!(
            "no se sobreescribira una ruta ajena: {}",
            path.display()
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| "archivo sin directorio".to_owned())?;
    fs::create_dir_all(parent).map_err(io_error("crear directorio de integracion"))?;
    let temporary = parent.join(format!(
        ".{}.gekkoapp-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file"),
        std::process::id()
    ));
    let _ = fs::remove_file(&temporary);
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(io_error("crear archivo temporal"))?;
    file.write_all(bytes)
        .map_err(io_error("escribir archivo temporal"))?;
    file.sync_all()
        .map_err(io_error("sincronizar archivo temporal"))?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o644))
        .map_err(io_error("aplicar permisos"))?;
    fs::rename(&temporary, path).map_err(io_error("activar archivo"))?;
    Ok(())
}

fn load_state(path: &Path) -> Result<InstallationState, String> {
    if !path.exists() {
        return Ok(InstallationState {
            schema_version: STATE_SCHEMA_VERSION,
            modules: BTreeMap::new(),
        });
    }
    let bytes = fs::read(path).map_err(io_error("leer estado de instalacion"))?;
    let state: InstallationState = serde_json::from_slice(&bytes)
        .map_err(|error| format!("estado de instalacion invalido: {error}"))?;
    if state.schema_version != STATE_SCHEMA_VERSION {
        return Err(format!(
            "version de estado no soportada: {}",
            state.schema_version
        ));
    }
    Ok(state)
}

fn write_state(path: &Path, state: &InstallationState) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|error| format!("no se pudo serializar el estado: {error}"))?;
    let owned = path.exists();
    write_owned_file(path, &bytes, owned)
}

/// Registra un componente instalado desde codigo fuente (p. ej. Gekko ADB
/// Studio) en el estado de GekkoApp sin pasar por un release firmado.
pub(crate) fn record_source_module(
    paths: &InstallPaths,
    module_id: &str,
    version: &str,
    active_root: &str,
    entrypoints: BTreeMap<String, String>,
) -> Result<(), String> {
    fs::create_dir_all(&paths.state_home).map_err(io_error("crear directorio de estado"))?;
    let mut state = load_state(&paths.state_file())?;
    state.modules.insert(
        module_id.to_string(),
        InstalledModule {
            version: version.to_string(),
            contract_version: "source".to_string(),
            active_root: active_root.to_string(),
            manifest_url: String::new(),
            artifact_url: String::new(),
            artifact_sha256: String::new(),
            entrypoints,
            owned_paths: Vec::new(),
            activated_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        },
    );
    write_state(&paths.state_file(), &state)
}

/// ¿Tiene GekkoApp registrado este modulo en su estado de instalacion?
///
/// Sirve para distinguir lo que instalo GekkoApp de lo que ya estaba en el
/// sistema antes de retirar algo cuyo nombre podria pertenecer a otro.
pub(crate) fn is_module_registered(module_id: &str) -> bool {
    let Ok(paths) = InstallPaths::detect() else {
        return false;
    };
    load_state(&paths.state_file())
        .map(|state| state.modules.contains_key(module_id))
        .unwrap_or(false)
}

/// Desinstala un modulo registrado limpiando sus owned_paths, active_root si
/// pertenece a kitotsu, y removiendolo del estado.
pub fn uninstall_registered_module(module_id: &str) -> Result<(), String> {
    let paths = InstallPaths::detect()?;
    let state_file = paths.state_file();
    let mut state = load_state(&state_file)?;

    if let Some(module) = state.modules.remove(module_id) {
        // Los fallos de borrado se acumulan y se informan: antes se ignoraban
        // todos y el flujo anunciaba una desinstalacion limpia que no lo era.
        let mut failures = Vec::new();
        for owned in &module.owned_paths {
            let path = Path::new(&owned.path);
            if fs::symlink_metadata(path).is_err() {
                continue;
            }
            if let Err(error) = fs::remove_file(path) {
                failures.push(format!("{}: {error}", owned.path));
            }
        }
        let root = Path::new(&module.active_root);
        if root.starts_with(&paths.versions_home) && root.exists() {
            if let Err(error) = fs::remove_dir_all(root) {
                failures.push(format!("{}: {error}", module.active_root));
            }
        }
        write_state(&state_file, &state)?;
        if !failures.is_empty() {
            return Err(format!(
                "{module_id} se retiro del estado, pero no se pudieron borrar: {}",
                failures.join("; ")
            ));
        }
    }

    Ok(())
}

fn validate_manifest(
    manifest: &ArtifactManifest,
    component_label: &str,
    expected_product_id: &str,
    expected_repository: &str,
    expected_tag: &str,
    expected_target: &str,
) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.kind != "kitotsu.release-artifact"
        || manifest.distribution_contract != "1.0"
    {
        return Err("contrato de distribucion no soportado".into());
    }
    if manifest.product.id != expected_product_id
        || manifest.product.repository != expected_repository
    {
        return Err(format!(
            "identidad de producto invalida para {}",
            component_label
        ));
    }
    if manifest.release.tag != expected_tag
        || manifest.product.version != expected_tag.trim_start_matches('v')
        || manifest.release.channel != "stable"
    {
        return Err(format!("version o canal invalido para {}", component_label));
    }
    if manifest.platform.os != "linux"
        || manifest.platform.arch != "x86_64"
        || manifest.platform.target != expected_target
        || manifest.platform.libc.family != "glibc"
    {
        return Err(format!("plataforma incompatible para {}", component_label));
    }
    if manifest
        .platform
        .libc
        .minimum
        .as_deref()
        .is_none_or(str::is_empty)
    {
        return Err("el manifiesto no declara glibc minima".into());
    }
    if manifest.artifact.format != "tar.zst"
        || manifest.artifact.size_bytes == 0
        || manifest.artifact.size_bytes > ARTIFACT_LIMIT
        || !is_sha256(&manifest.artifact.sha256)
        || !safe_file_name(&manifest.artifact.file_name)
    {
        return Err("descripcion de artefacto invalida".into());
    }
    if manifest.payload.is_empty() || manifest.entrypoints.is_empty() {
        return Err("payload o entrypoints vacios".into());
    }
    for capability in &manifest.requirements.host_capabilities {
        if !capability.optional
            && !matches!(
                capability.id.as_str(),
                "session.wayland" | "runtime.qt6" | "renderer.awww" | "gpu.wgpu" | "audio.pipewire"
            )
        {
            return Err(format!(
                "capacidad obligatoria no soportada por GekkoApp: {}",
                capability.id
            ));
        }
    }
    let mut payload_paths = BTreeSet::new();
    for entry in &manifest.payload {
        validate_manifest_path(&entry.path)?;
        if !payload_paths.insert(entry.path.as_str())
            || !is_sha256(&entry.sha256)
            || !matches!(
                entry.kind.as_str(),
                "executable"
                    | "library"
                    | "resource"
                    | "desktop-entry-template"
                    | "icon"
                    | "completion"
                    | "license"
                    | "migration"
            )
        {
            return Err(format!("entrada payload invalida: {}", entry.path));
        }
    }
    for entrypoint in &manifest.entrypoints {
        validate_entrypoint_name(&entrypoint.name)?;
        if !payload_paths.contains(entrypoint.path.as_str()) {
            return Err(format!("entrypoint fuera de payload: {}", entrypoint.path));
        }
    }
    for desktop in &manifest.integrations.desktop_entries {
        validate_application_id(&desktop.application_id)?;
        validate_manifest_path(&desktop.template)?;
        if !payload_paths.contains(desktop.template.as_str()) {
            return Err(format!(
                "plantilla desktop fuera de payload: {}",
                desktop.template
            ));
        }
        if !manifest
            .entrypoints
            .iter()
            .any(|entrypoint| entrypoint.name == desktop.entrypoint)
        {
            return Err(format!(
                "entrypoint desktop desconocido: {}",
                desktop.entrypoint
            ));
        }
        for icon in &desktop.icons {
            validate_manifest_path(&icon.source)?;
            if !payload_paths.contains(icon.source.as_str()) {
                return Err(format!("icono fuera de payload: {}", icon.source));
            }
        }
    }
    Ok(())
}

fn validate_module_dependencies(releases: &[PreparedRelease]) -> Result<(), String> {
    let available = releases
        .iter()
        .map(|release| release.manifest.product.id.as_str())
        .collect::<BTreeSet<_>>();
    for release in releases {
        for requirement in &release.manifest.requirements.modules {
            if !requirement.optional && !available.contains(requirement.id.as_str()) {
                return Err(format!(
                    "{} requiere el modulo {}",
                    release.manifest.product.id, requirement.id
                ));
            }
        }
    }
    Ok(())
}

fn detect_glibc_version() -> Result<String, String> {
    let output = Command::new("getconf")
        .arg("GNU_LIBC_VERSION")
        .output()
        .map_err(|error| format!("no se pudo consultar glibc con getconf: {error}"))?;
    if !output.status.success() {
        return Err("getconf no pudo determinar la version de glibc".into());
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("version de glibc no UTF-8: {error}"))?;
    stdout
        .split_whitespace()
        .find(|value| parse_version(value).is_ok())
        .map(str::to_owned)
        .ok_or_else(|| format!("respuesta de glibc no reconocida: {}", stdout.trim()))
}

pub(crate) fn compare_versions(left: &str, right: &str) -> Result<std::cmp::Ordering, String> {
    let left = parse_version(left)?;
    let right = parse_version(right)?;
    let length = left.len().max(right.len());
    Ok((0..length)
        .map(|index| {
            left.get(index)
                .copied()
                .unwrap_or_default()
                .cmp(&right.get(index).copied().unwrap_or_default())
        })
        .find(|ordering| *ordering != std::cmp::Ordering::Equal)
        .unwrap_or(std::cmp::Ordering::Equal))
}

fn parse_version(value: &str) -> Result<Vec<u32>, String> {
    if value.is_empty() {
        return Err("version vacia".into());
    }
    // Se separa la version publica de la etiqueta local de PEP 440
    // (`0.10.8+gekko.1` -> publica `0.10.8`, local `gekko.1`) y se ignora
    // cualquier sufijo no numerico de un componente (`2.41-rc1` -> `2.41`).
    // Sin esto la campana de actualizaciones no podia comparar las versiones
    // del fork de Bauh, que usa exactamente ese esquema.
    let (public, local) = match value.split_once('+') {
        Some((public, local)) => (public, Some(local)),
        None => (value, None),
    };
    let mut parts = Vec::new();
    for component in public.split('.') {
        let digits = component
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        if digits.is_empty() {
            // Un componente que ni siquiera empieza por un digito ("2.x") sigue
            // siendo una version invalida: la tolerancia es solo para sufijos.
            return Err(format!("version invalida: {value}"));
        }
        parts.push(
            digits
                .parse::<u32>()
                .map_err(|_| format!("version invalida: {value}"))?,
        );
        if digits.len() != component.len() {
            break;
        }
    }
    if parts.is_empty() {
        return Err(format!("version invalida: {value}"));
    }
    // Los numeros de la etiqueta local se anaden detras, para que
    // `0.10.8+gekko.2` > `0.10.8+gekko.1` > `0.10.8`, como manda PEP 440. Los
    // segmentos no numericos de la etiqueta (`gekko`) no participan en el
    // orden: basta con distinguir builds del mismo upstream.
    if let Some(local) = local {
        for component in local.split('.') {
            let digits = component
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>();
            if let Ok(number) = digits.parse::<u32>() {
                parts.push(number);
            }
        }
    }
    Ok(parts)
}

fn validate_manifest_path(path: &str) -> Result<(), String> {
    if path.split('/').any(str::is_empty) || path.contains('\\') {
        return Err(format!("ruta relativa no permitida: {path}"));
    }
    validate_relative_path(Path::new(path))
}

fn validate_relative_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("ruta relativa no permitida: {}", path.display()));
    }
    Ok(())
}

fn validate_entrypoint_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        return Err(format!("nombre de entrypoint invalido: {name}"));
    }
    Ok(())
}

fn validate_application_id(id: &str) -> Result<(), String> {
    if id.split('.').count() < 3
        || !id.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
    {
        return Err(format!("application ID invalido: {id}"));
    }
    Ok(())
}

pub(crate) fn ensure_https(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err(format!("solo se permiten descargas HTTPS: {url}"));
    }
    Ok(())
}

fn safe_file_name(name: &str) -> bool {
    // Se admite '+' porque las versiones locales de PEP 440 (por ejemplo
    // `0.10.8+gekko.1`, la que usa el fork de Bauh) viajan en el nombre del
    // artefacto. Sigue sin admitirse cualquier separador de ruta.
    !name.is_empty()
        && Path::new(name).file_name().and_then(|value| value.to_str()) == Some(name)
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-+".contains(&byte))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(io_error("abrir archivo para hash"))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(io_error("leer archivo para hash"))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn file_size(path: &Path) -> Result<u64, String> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(io_error("leer tamano de archivo"))
}

fn path_to_manifest_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| format!("ruta no UTF-8: {}", path.display()))
}

fn path_text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("ruta no UTF-8: {}", path.display()))
}

fn component_text<'a>(component: Component<'a>) -> Option<&'a str> {
    match component {
        Component::Normal(value) => value.to_str(),
        _ => None,
    }
}

fn desktop_exec_quote(path: &str) -> String {
    format!(
        "\"{}\"",
        path.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$")
    )
}

fn kiui_runtime_environment(paths: &InstallPaths) -> Result<String, String> {
    let variables = [
        ("KIUI_COMPOSITOR_BIN", "kitsune-compositor"),
        ("KITSUNE_COMPOSITOR_BIN", "kitsune-compositor"),
        ("KIUI_KITOWALL_BIN", "kitowall"),
        ("KIUI_KILIVEPAPER_BIN", "kilivepaper"),
        ("KIUI_KISDDM_BIN", "kisddm"),
        ("KIUI_KITSUNE_BIN", "kitsune"),
    ];
    let assignments = variables
        .into_iter()
        .map(|(variable, binary)| {
            path_text(&paths.bin_home.join(binary))
                .map(|path| format!("{variable}={}", desktop_exec_quote(&path)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!("/usr/bin/env {}", assignments.join(" ")))
}

fn now_unix() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("reloj del sistema invalido: {error}"))
}

fn io_error(context: &'static str) -> impl FnOnce(io::Error) -> String {
    move |error| format!("{context}: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kito::ComponentId;
    use std::io::Cursor;

    #[test]
    fn rejects_unsafe_payload_paths() {
        assert!(validate_manifest_path("bin/kiui").is_ok());
        assert!(validate_manifest_path("../bin/kiui").is_err());
        assert!(validate_manifest_path("/usr/bin/kiui").is_err());
        assert!(validate_manifest_path("bin//kiui").is_err());
    }

    #[test]
    fn maps_only_required_host_capabilities_to_host_packages() {
        let manifest = test_manifest();
        let plan = InstallationPlan {
            releases: vec![PreparedRelease {
                component_label: ComponentId::Kiui.label().to_string(),
                manifest_url: "https://example.test/manifest.json".into(),
                artifact_url: "https://example.test/archive.tar.zst".into(),
                manifest,
            }],
        };
        let (arch_packages, arch_unsupported) = plan.required_host_packages(false);
        assert_eq!(
            arch_packages,
            vec!["qt6-base", "qt6-declarative", "qt6-wayland"]
        );
        assert!(arch_unsupported.is_empty());

        // Los nombres de Qt6 coinciden en Solus, asi que el plan es el mismo.
        let (solus_packages, solus_unsupported) = plan.required_host_packages(true);
        assert_eq!(solus_packages, arch_packages);
        assert!(solus_unsupported.is_empty());
    }

    #[test]
    fn desktop_exec_paths_are_quoted_and_escaped() {
        assert_eq!(
            desktop_exec_quote("/home/Kito User/$bin/kiui"),
            "\"/home/Kito User/\\$bin/kiui\""
        );
    }

    #[test]
    #[ignore = "requiere GEKKOAPP_BAUH_DIST apuntando a un release generado por scripts/build-bauh-release.sh"]
    fn consumes_a_generated_pipx_bauh_release() {
        let dist = std::env::var("GEKKOAPP_BAUH_DIST").expect("GEKKOAPP_BAUH_DIST sin definir");
        let dist = Path::new(&dist);
        let manifest_path = fs::read_dir(dist)
            .expect("leer dist")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "json"))
            .expect("falta el manifiesto en dist");
        let archive_path = fs::read_dir(dist)
            .expect("leer dist")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "zst"))
            .expect("falta el artefacto en dist");

        let bytes = fs::read(&manifest_path).expect("leer manifiesto");
        let manifest: ArtifactManifest =
            serde_json::from_slice(&bytes).expect("manifiesto JSON invalido");
        assert_eq!(manifest.install_method, "python_pipx");

        let tag = manifest.release.tag.clone();
        let target = manifest.platform.target.clone();
        validate_manifest(
            &manifest,
            crate::core::catalog::BAUH_LABEL,
            crate::core::catalog::BAUH_PRODUCT_ID,
            crate::core::catalog::BAUH_REPOSITORY,
            &tag,
            &target,
        )
        .expect("el manifiesto debe pasar la validacion del motor");

        let root = env::temp_dir().join(format!(
            "gekkoapp-bauh-contract-{}-{}",
            std::process::id(),
            now_unix().unwrap()
        ));
        fs::create_dir_all(&root).unwrap();
        let paths = InstallPaths {
            bin_home: root.join("bin"),
            data_home: root.join("data"),
            state_home: root.join("state"),
            cache_home: root.join("cache"),
            versions_home: root.join("versions"),
        };

        let release = PreparedRelease {
            component_label: crate::core::catalog::BAUH_LABEL.to_string(),
            manifest_url: "https://example.test/manifest.json".into(),
            artifact_url: "https://example.test/archive.tar.zst".into(),
            manifest: manifest.clone(),
        };
        let installed_root = install_version(&release, &archive_path, &paths)
            .expect("la version debe extraerse y verificarse contra el payload");

        let desktop = &manifest.integrations.desktop_entries[0];
        let entrypoints: BTreeMap<String, String> = manifest
            .entrypoints
            .iter()
            .map(|entrypoint| (entrypoint.name.clone(), "/tmp/.local/bin/bauh".to_string()))
            .collect();
        let mut owned = Vec::new();
        activate_desktop_integration(
            desktop,
            &installed_root,
            &paths,
            &entrypoints,
            &BTreeSet::new(),
            &mut owned,
        )
        .expect("la integracion desktop debe materializarse");

        let desktop_file = paths
            .data_home
            .join("applications")
            .join(format!("{}.desktop", desktop.application_id));
        let content = fs::read_to_string(&desktop_file).expect("leer .desktop generado");
        assert!(content.contains("Exec=\"/tmp/.local/bin/bauh\""));
        assert!(!content.contains('@'));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn kiui_desktop_uses_absolute_module_entrypoints() {
        let paths = InstallPaths {
            bin_home: PathBuf::from("/home/Kito User/.local/bin"),
            data_home: PathBuf::from("/tmp/data"),
            state_home: PathBuf::from("/tmp/state"),
            cache_home: PathBuf::from("/tmp/cache"),
            versions_home: PathBuf::from("/tmp/versions"),
        };
        let environment = kiui_runtime_environment(&paths).unwrap();

        assert!(environment.starts_with("/usr/bin/env "));
        assert!(environment
            .contains("KIUI_COMPOSITOR_BIN=\"/home/Kito User/.local/bin/kitsune-compositor\""));
        assert!(environment
            .contains("KITSUNE_COMPOSITOR_BIN=\"/home/Kito User/.local/bin/kitsune-compositor\""));
        assert!(
            environment.contains("KIUI_KILIVEPAPER_BIN=\"/home/Kito User/.local/bin/kilivepaper\"")
        );
        assert!(environment.contains("KIUI_KISDDM_BIN=\"/home/Kito User/.local/bin/kisddm\""));
    }

    #[test]
    fn compares_numeric_versions() {
        assert_eq!(
            compare_versions("2.43", "2.39").unwrap(),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_versions("2.39", "2.39.0").unwrap(),
            std::cmp::Ordering::Equal
        );
        assert!(compare_versions("2.x", "2.39").is_err());

        // Versiones locales de PEP 440: el fork de Bauh publica `0.10.8+gekko.1`
        // y la campana de actualizaciones tiene que poder compararlas.
        assert_eq!(
            compare_versions("0.10.8+gekko.1", "0.10.7").unwrap(),
            std::cmp::Ordering::Greater
        );
        // La etiqueta local ordena por detras de la version publica.
        assert_eq!(
            compare_versions("0.10.8+gekko.2", "0.10.8").unwrap(),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_versions("0.10.8+gekko.2", "0.10.8+gekko.1").unwrap(),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_versions("0.10.8+gekko.1", "0.10.8+gekko.1").unwrap(),
            std::cmp::Ordering::Equal
        );
        // ...pero nunca por delante de una version publica mayor.
        assert_eq!(
            compare_versions("0.10.8+gekko.9", "0.10.9").unwrap(),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_versions("1.0.0-rc1", "1.0.0").unwrap(),
            std::cmp::Ordering::Equal
        );
        assert!(compare_versions("", "1.0").is_err());
        assert!(compare_versions("+gekko.1", "1.0").is_err());
    }

    #[test]
    fn installs_and_activates_a_verified_version_from_tar_zstd() {
        let root = env::temp_dir().join(format!(
            "gekkoapp-installer-test-{}-{}",
            std::process::id(),
            now_unix().unwrap()
        ));
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("kiui.tar.zst");
        let executable = b"#!/bin/sh\nexit 0\n";
        create_test_archive(&archive_path, executable);

        let mut manifest = test_manifest();
        manifest.payload[0].size_bytes = executable.len() as u64;
        manifest.payload[0].sha256 = sha256_bytes(executable);
        let release = PreparedRelease {
            component_label: ComponentId::Kiui.label().to_string(),
            manifest_url: "https://example.test/manifest.json".into(),
            artifact_url: "https://example.test/archive.tar.zst".into(),
            manifest,
        };
        let paths = InstallPaths {
            bin_home: root.join("bin"),
            data_home: root.join("data"),
            state_home: root.join("state"),
            cache_home: root.join("cache"),
            versions_home: root.join("versions"),
        };

        let installed = install_version(&release, &archive_path, &paths).unwrap();
        assert_eq!(fs::read(installed.join("bin/kiui")).unwrap(), executable);
        let mut state = InstallationState::default();
        activate_release(&release, &installed, &paths, &mut state).unwrap();
        write_state(&paths.state_file(), &state).unwrap();
        assert_eq!(
            fs::read_link(paths.bin_home.join("kiui")).unwrap(),
            installed.join("bin/kiui")
        );
        assert!(paths.state_file().is_file());
        assert_eq!(state.modules["kiui"].version, "0.1.1");
        fs::remove_dir_all(root).unwrap();
    }

    fn create_test_archive(path: &Path, executable: &[u8]) {
        let file = File::create(path).unwrap();
        let encoder = zstd::stream::write::Encoder::new(file, 1).unwrap();
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_path("kiui-0.1.1/bin/kiui").unwrap();
        header.set_size(executable.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        archive.append(&header, Cursor::new(executable)).unwrap();
        let encoder = archive.into_inner().unwrap();
        encoder.finish().unwrap();
    }

    fn sha256_bytes(bytes: &[u8]) -> String {
        let mut digest = Sha256::new();
        digest.update(bytes);
        format!("{:x}", digest.finalize())
    }

    fn test_manifest() -> ArtifactManifest {
        serde_json::from_value(serde_json::json!({
            "schema_version": 1,
            "kind": "kitotsu.release-artifact",
            "distribution_contract": "1.0",
            "product": {"id": "kiui", "version": "0.1.1", "repository": "KitotsuMolina/KiUI", "contract_version": "1.0"},
            "release": {"tag": "v0.1.1", "channel": "stable"},
            "platform": {"os": "linux", "arch": "x86_64", "target": "x86_64-unknown-linux-gnu", "libc": {"family": "glibc", "minimum": "2.39"}},
            "artifact": {"file_name": "kiui.tar.zst", "format": "tar.zst", "size_bytes": 1, "sha256": "0".repeat(64)},
            "payload": [{"path": "bin/kiui", "kind": "executable", "mode": "0755", "size_bytes": 1, "sha256": "0".repeat(64)}],
            "entrypoints": [{"name": "kiui", "path": "bin/kiui"}],
            "requirements": {"modules": [], "host_capabilities": [{"id": "runtime.qt6", "optional": false}]},
            "integrations": {"desktop_entries": []}
        }))
        .unwrap()
    }
}
