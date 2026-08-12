//! Interfaz de escritorio Tauri v2 (Control Center).
//!
//! Expone el catalogo y la instalacion de componentes (Kito, Bauh Fork, Gekko
//! ADB Studio, GekkoApp) y de los modulos del post-install (Terminal, Hyprland,
//! Niri, Gaming, Chaotic AUR) al frontend `ui/` a traves de comandos. El
//! progreso de los flujos se emite como eventos `install://event` mediante un
//! [`GuiReporter`]. El comando `check_updates` alimenta la campana de
//! actualizaciones del Control Center.
//!
//! Elevacion de privilegios: la GUI no tiene TTY, asi que la contrasena de
//! sudo se entrega a un helper `askpass` temporal (`SUDO_ASKPASS` +
//! `GEKKOAPP_ASKPASS`) creado con permisos 0600/0700 y eliminado al terminar.
use crate::core::catalog::{all_components, CatalogComponent};
use crate::core::reporter::Reporter;
use crate::environment::SystemEnvironment;
use crate::installer::InstallPaths;
use crate::kito::{ComponentId, ModuleSelection};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

// ─────────────────────────────────────────────────────────────────────────────
//  Reporter que reenvia el progreso al frontend
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "data")]
pub enum InstallEvent {
    Log {
        level: &'static str,
        message: String,
    },
    Progress {
        label: String,
        percent: u32,
    },
}

#[derive(Clone)]
pub struct GuiReporter {
    app: AppHandle,
}

impl GuiReporter {
    fn send(&self, event: InstallEvent) {
        let _ = self.app.emit("install://event", event);
    }
}

impl Reporter for GuiReporter {
    fn ok(&self, msg: &str) {
        self.send(InstallEvent::Log {
            level: "ok",
            message: msg.to_string(),
        });
    }

    fn warn(&self, msg: &str) {
        self.send(InstallEvent::Log {
            level: "warn",
            message: msg.to_string(),
        });
    }

    fn info(&self, msg: &str) {
        self.send(InstallEvent::Log {
            level: "info",
            message: msg.to_string(),
        });
    }

    fn err(&self, msg: &str) {
        self.send(InstallEvent::Log {
            level: "err",
            message: msg.to_string(),
        });
    }

    fn step(&self, msg: &str) {
        self.send(InstallEvent::Log {
            level: "step",
            message: msg.to_string(),
        });
    }

    fn header(&self, title: &str) {
        self.send(InstallEvent::Log {
            level: "header",
            message: title.to_string(),
        });
    }

    fn confirm(&self, _prompt: &str) -> bool {
        true
    }

    fn prompt(&self, _label: &str, current: &str) -> String {
        current.to_string()
    }

    fn read_line(&self) -> String {
        String::new()
    }

    fn clear_screen(&self) {}

    fn progress(&self, label: &str, _steps: u32) {
        self.send(InstallEvent::Progress {
            label: label.to_string(),
            percent: 100,
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Catalogo (estado local + entorno)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    id: &'static str,
    label: &'static str,
    repository: &'static str,
    installed_version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleView {
    product_id: &'static str,
    label: &'static str,
    mandatory: bool,
    installed_version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogView {
    distro_id: String,
    distro_name: String,
    package_manager: String,
    session: String,
    desktop: String,
    target: Option<&'static str>,
    compatible: bool,
    items: Vec<CatalogItem>,
    kito_modules: Vec<ModuleView>,
}

fn installed_versions() -> BTreeMap<String, String> {
    let Ok(paths) = InstallPaths::detect() else {
        return BTreeMap::new();
    };
    let Ok(text) = std::fs::read_to_string(paths.state_file()) else {
        return BTreeMap::new();
    };
    let Ok(state) = serde_json::from_str::<crate::installer::InstallationState>(&text) else {
        return BTreeMap::new();
    };
    state
        .modules
        .into_iter()
        .map(|(id, module)| (id, module.version))
        .collect()
}

fn is_binary_in_path(name: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if dir.join(name).exists() {
                return true;
            }
        }
    }
    false
}

/// Version instalada de un componente del catalogo.
///
/// Primero consulta el estado interno (`installations-v1.json`). Si no esta
/// registrado (por ejemplo, si Bauh o Gekko ADB fueron instalados por pipx,
/// por curl o previo a GekkoApp), realiza una comprobacion en vivo del binario
/// en `~/.local/bin` o PATH en Arch, Garuda y Solus.
fn installed_version_of(
    installed: &BTreeMap<String, String>,
    component: CatalogComponent,
) -> Option<String> {
    if let Some(version) = installed.get(component.id()) {
        return Some(version.clone());
    }

    let Ok(paths) = InstallPaths::detect() else {
        return None;
    };

    match component {
        CatalogComponent::BauhFork => {
            let launcher = paths.bin_home.join("bauh");
            if launcher.exists() || is_binary_in_path("bauh") {
                let cmd = if launcher.exists() {
                    crate::core::system::sh_quote(&launcher)
                } else {
                    "bauh".to_string()
                };
                let (_, version_output) =
                    crate::core::system::run_shell_piped(&format!("{} --version 2>/dev/null", cmd));
                let v = version_output.trim();
                if let Some(ver) = v.strip_prefix("bauh ") {
                    let ver = ver.trim();
                    if !ver.is_empty() {
                        return Some(ver.to_string());
                    }
                }
                if !v.is_empty() {
                    return Some(v.to_string());
                }
                return Some("instalado".to_string());
            }
            None
        }
        CatalogComponent::GekkoAdb => {
            let launcher = paths.bin_home.join("gekko-adb");
            let app_dir = paths.data_home.join("gekko-adb/app");
            if launcher.exists() || app_dir.exists() || is_binary_in_path("gekko-adb") {
                if app_dir.join(".git").exists() {
                    let (_, rev) = crate::core::system::run_shell_piped(&format!(
                        "git -C {} rev-parse --short HEAD 2>/dev/null",
                        crate::core::system::sh_quote(&app_dir)
                    ));
                    let rev = rev.trim();
                    if !rev.is_empty() {
                        return Some(rev.to_string());
                    }
                }
                return Some("instalado".to_string());
            }
            None
        }

        CatalogComponent::GekkoApp => Some(env!("CARGO_PKG_VERSION").to_string()),
        CatalogComponent::Kito(kito_comp) => {
            let binary_name = match kito_comp {
                ComponentId::Compositor => "kitsune-compositor",
                ComponentId::Kiui => "kiui",
                ComponentId::Kitowall => "kitowall",
                ComponentId::Kilivepaper => "kilivepaper",
                ComponentId::Kisddm => "kisddm",
            };
            let launcher = paths.bin_home.join(binary_name);
            if launcher.exists() || is_binary_in_path(binary_name) {
                return Some("instalado".to_string());
            }
            None
        }
    }
}

#[tauri::command]
fn catalog_state() -> CatalogView {
    let environment = SystemEnvironment::detect();
    let installed = installed_versions();

    let items = all_components()
        .into_iter()
        .map(|component| CatalogItem {
            id: component.id(),
            label: component.label(),
            repository: component.repository(),
            installed_version: installed_version_of(&installed, component),
        })
        .collect();

    let kito_modules = all_components()
        .into_iter()
        .filter_map(|component| match component {
            CatalogComponent::Kito(component) => Some(ModuleView {
                product_id: component.product_id(),
                label: component.label(),
                mandatory: matches!(component, ComponentId::Compositor | ComponentId::Kiui),
                installed_version: installed_version_of(
                    &installed,
                    CatalogComponent::Kito(component),
                ),
            }),
            CatalogComponent::BauhFork
            | CatalogComponent::GekkoAdb
            | CatalogComponent::GekkoApp => None,
        })
        .collect();

    let target = environment.target();

    CatalogView {
        distro_id: environment.distro_id,
        distro_name: environment.distro_name,
        package_manager: environment.package_manager,
        session: environment.session,
        desktop: environment.desktop,
        target,
        compatible: environment.compatibility.supported,
        items,
        kito_modules,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Comprobacion de actualizaciones (campana del Control Center)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    id: &'static str,
    label: &'static str,
    installed: Option<String>,
    latest: Option<String>,
    update_available: bool,
}

/// Consulta la ultima version publicada de cada componente del catalogo con
/// releases firmados (Kito, Bauh Fork y el propio GekkoApp; Gekko ADB Studio
/// no publica releases todavia) y la compara con la instalada localmente.
#[tauri::command]
async fn check_updates() -> Result<Vec<UpdateInfo>, String> {
    let environment = SystemEnvironment::detect();
    let Some(target) = environment.target() else {
        return Ok(Vec::new());
    };
    let installed = installed_versions();
    tauri::async_runtime::spawn_blocking(move || {
        let mut updates = Vec::new();
        for component in all_components() {
            if matches!(component, CatalogComponent::GekkoAdb) {
                continue;
            }
            let installed_version = installed_version_of(&installed, component);
            let latest =
                crate::core::github::resolve_latest_release(component.repository(), target)
                    .ok()
                    .map(|(tag, _, _)| tag.trim_start_matches('v').to_string());
            let update_available = match (&installed_version, &latest) {
                (Some(current), Some(newer)) => crate::installer::compare_versions(newer, current)
                    .map(|ordering| ordering == std::cmp::Ordering::Greater)
                    .unwrap_or(false),
                _ => false,
            };
            updates.push(UpdateInfo {
                id: component.id(),
                label: component.label(),
                installed: installed_version,
                latest,
                update_available,
            });
        }
        Ok(updates)
    })
    .await
    .map_err(|error| format!("La comprobacion de actualizaciones aborto: {error}"))?
}

// ─────────────────────────────────────────────────────────────────────────────
//  Askpass: contrasena de sudo sin TTY
// ─────────────────────────────────────────────────────────────────────────────

struct AskpassGuard {
    dir: PathBuf,
}

impl AskpassGuard {
    fn setup(password: &str) -> Result<Self, String> {
        let dir = std::env::temp_dir().join(format!(
            "gekkoapp-askpass-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).map_err(|error| format!("No se pudo crear {dir:?}: {error}"))?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("No se pudieron fijar permisos en {dir:?}: {error}"))?;

        let password_file = dir.join("password");
        let mut password_handle = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&password_file)
            .map_err(|error| format!("No se pudo crear la contrasena: {error}"))?;
        password_handle
            .write_all(password.as_bytes())
            .map_err(|error| format!("No se pudo escribir la contrasena: {error}"))?;
        password_handle
            .sync_all()
            .map_err(|error| format!("No se pudo sincronizar la contrasena: {error}"))?;

        let helper = dir.join("askpass.sh");
        std::fs::write(
            &helper,
            format!("#!/bin/sh\ncat \"{}\"\n", password_file.display()),
        )
        .map_err(|error| format!("No se pudo crear el helper askpass: {error}"))?;
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).ok();

        std::env::set_var("SUDO_ASKPASS", &helper);
        std::env::set_var("GEKKOAPP_ASKPASS", &helper);
        Ok(Self { dir })
    }
}

impl Drop for AskpassGuard {
    fn drop(&mut self) {
        std::env::remove_var("SUDO_ASKPASS");
        std::env::remove_var("GEKKOAPP_ASKPASS");
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn require_password(password: Option<String>) -> Result<String, String> {
    password
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Introduce tu contrasena de sudo en la interfaz.".to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
//  Comandos de instalacion
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
async fn install_kito(
    app: AppHandle,
    selection: ModuleSelection,
    password: Option<String>,
) -> Result<usize, String> {
    let environment = SystemEnvironment::detect();
    if !environment.compatibility.supported {
        return Err(
            "La instalacion de Kito requiere una sesion Wayland sobre Hyprland en Arch."
                .to_string(),
        );
    }
    let password = require_password(password)?;
    let reporter = GuiReporter { app };
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = AskpassGuard::setup(&password)?;
        crate::core::flow::install_kito_plan(&reporter, environment, selection, false)
    })
    .await
    .map_err(|error| format!("La tarea de instalacion aborto: {error}"))?
}

#[tauri::command]
async fn install_bauh(app: AppHandle, password: Option<String>) -> Result<(), String> {
    let environment = SystemEnvironment::detect();
    let password = require_password(password)?;
    let reporter = GuiReporter { app };
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = AskpassGuard::setup(&password)?;
        crate::core::flow::install_bauh(&reporter, &environment, false)
    })
    .await
    .map_err(|error| format!("La tarea de instalacion aborto: {error}"))?
}

#[tauri::command]
async fn install_gekko_adb(app: AppHandle, password: Option<String>) -> Result<(), String> {
    run_gui_install(app, password, |reporter| {
        crate::core::flow::install_gekko_adb(reporter)
    })
    .await
}

/// Ejecuta un flujo de instalacion en un hilo bloqueante con askpass sudo.
async fn spawn_gui_install<T, F>(
    app: AppHandle,
    password: Option<String>,
    f: F,
) -> Result<T, String>
where
    F: FnOnce(&dyn Reporter) -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    let password = require_password(password)?;
    let reporter = GuiReporter { app };
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = AskpassGuard::setup(&password)?;
        f(&reporter)
    })
    .await
    .map_err(|error| format!("La tarea de instalacion aborto: {error}"))?
}

async fn run_gui_install<F>(app: AppHandle, password: Option<String>, f: F) -> Result<(), String>
where
    F: FnOnce(&dyn Reporter) -> Result<(), String> + Send + 'static,
{
    spawn_gui_install(app, password, f).await
}

#[tauri::command]
async fn install_terminal(app: AppHandle, password: Option<String>) -> Result<(), String> {
    run_gui_install(app, password, |reporter| {
        if crate::core::flow::install_zsh_starship(reporter) {
            Ok(())
        } else {
            Err("La instalacion de Terminal Bonita fallo.".to_string())
        }
    })
    .await
}

#[tauri::command]
async fn install_hyprland(app: AppHandle, password: Option<String>) -> Result<(), String> {
    run_gui_install(app, password, |reporter| {
        if crate::core::flow::install_hyprland(reporter) {
            Ok(())
        } else {
            Err("La instalacion del entorno Hyprland fallo.".to_string())
        }
    })
    .await
}

#[tauri::command]
async fn install_niri(app: AppHandle, password: Option<String>) -> Result<(), String> {
    run_gui_install(app, password, |reporter| {
        if crate::core::flow::install_niri(reporter) {
            Ok(())
        } else {
            Err("La instalacion del entorno Niri fallo.".to_string())
        }
    })
    .await
}

#[tauri::command]
async fn install_gaming_setup(
    app: AppHandle,
    gpu: String,
    password: Option<String>,
) -> Result<(), String> {
    let vulkan_choice = match gpu.as_str() {
        "nvidia" => "1",
        "intel" => "7",
        "amd" => "12",
        _ => return Err(format!("GPU no soportada: {gpu}")),
    };
    run_gui_install(app, password, move |reporter| {
        if crate::core::flow::install_gaming(reporter, &gpu, vulkan_choice) {
            Ok(())
        } else {
            Err("La instalacion de Gaming fallo.".to_string())
        }
    })
    .await
}

#[tauri::command]
async fn install_chaotic_aur(app: AppHandle, password: Option<String>) -> Result<(), String> {
    run_gui_install(app, password, |reporter| {
        if crate::core::pacman::install_chaotic_aur(reporter) {
            Ok(())
        } else {
            Err("No se pudo configurar Chaotic AUR.".to_string())
        }
    })
    .await
}

#[tauri::command]
async fn install_gekkoapp(app: AppHandle) -> Result<(), String> {
    let environment = SystemEnvironment::detect();
    let reporter = GuiReporter { app };
    tauri::async_runtime::spawn_blocking(move || {
        crate::core::flow::install_gekkoapp(&reporter, &environment, false)
    })
    .await
    .map_err(|error| format!("La tarea de actualizacion aborto: {error}"))?
}

// ─────────────────────────────────────────────────────────────────────────────
//  Theme (integracion con Matugen / Material You)
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
fn theme_state() -> crate::core::theme::MatugenPalette {
    crate::core::theme::detect_palette()
}

#[tauri::command]
async fn uninstall_kito(app: AppHandle) -> Result<(), String> {
    let reporter = GuiReporter { app };
    tauri::async_runtime::spawn_blocking(move || {
        crate::core::flow::uninstall_kito_environment(&reporter)
    })
    .await
    .map_err(|error| format!("Error al desinstalar Kito: {error}"))?
}

#[tauri::command]
async fn uninstall_bauh(app: AppHandle) -> Result<(), String> {
    let reporter = GuiReporter { app };
    tauri::async_runtime::spawn_blocking(move || crate::core::flow::uninstall_bauh(&reporter))
        .await
        .map_err(|error| format!("Error al desinstalar Bauh: {error}"))?
}

#[tauri::command]
async fn uninstall_gekko_adb(app: AppHandle) -> Result<(), String> {
    let reporter = GuiReporter { app };
    tauri::async_runtime::spawn_blocking(move || crate::core::flow::uninstall_gekko_adb(&reporter))
        .await
        .map_err(|error| format!("Error al desinstalar Gekko ADB Studio: {error}"))?
}

#[tauri::command]
async fn uninstall_terminal(app: AppHandle) -> Result<(), String> {
    let reporter = GuiReporter { app };
    tauri::async_runtime::spawn_blocking(move || {
        if crate::core::flow::uninstall_zsh_starship(&reporter) {
            Ok(())
        } else {
            Err("Error al desinstalar Terminal Bonita.".to_string())
        }
    })
    .await
    .map_err(|error| format!("Error en la tarea: {error}"))?
}

#[tauri::command]
async fn uninstall_hyprland(app: AppHandle, password: Option<String>) -> Result<(), String> {
    run_gui_install(app, password, |reporter| {
        if crate::core::flow::uninstall_hyprland(reporter) {
            Ok(())
        } else {
            Err("Error al desinstalar preset Hyprland.".to_string())
        }
    })
    .await
}

#[tauri::command]
async fn uninstall_niri(app: AppHandle, password: Option<String>) -> Result<(), String> {
    run_gui_install(app, password, |reporter| {
        if crate::core::flow::uninstall_niri(reporter) {
            Ok(())
        } else {
            Err("Error al desinstalar preset Niri.".to_string())
        }
    })
    .await
}

#[tauri::command]
async fn uninstall_gaming_setup(app: AppHandle, password: Option<String>) -> Result<(), String> {
    run_gui_install(app, password, |reporter| {
        if crate::core::flow::uninstall_gaming(reporter) {
            Ok(())
        } else {
            Err("Error al desinstalar Gaming Setup.".to_string())
        }
    })
    .await
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                crate::core::theme::watch_palette(move |palette| {
                    let _ = handle.emit("theme://changed", palette);
                });
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            catalog_state,
            check_updates,
            install_kito,
            install_bauh,
            install_gekko_adb,
            install_gekkoapp,
            install_terminal,
            install_hyprland,
            install_niri,
            install_gaming_setup,
            install_chaotic_aur,
            uninstall_kito,
            uninstall_bauh,
            uninstall_gekko_adb,
            uninstall_terminal,
            uninstall_hyprland,
            uninstall_niri,
            uninstall_gaming_setup,
            theme_state
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar la interfaz GekkoApp");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_state_reports_all_components() {
        let view = catalog_state();

        assert_eq!(view.items.len(), 8);
        assert!(view
            .items
            .iter()
            .any(|item| item.id == "kitsune-compositor"));
        assert!(view
            .items
            .iter()
            .any(|item| item.id == "bauh-fork-the-gekko"));
        assert!(view.items.iter().any(|item| item.id == "gekko-adb"));
        assert!(view.items.iter().any(|item| item.id == "gekkoapp"));
        assert!(view
            .items
            .iter()
            .any(|item| item.id == "gekkoapp" && item.installed_version.is_some()));
        assert_eq!(view.kito_modules.len(), 5);
        assert!(view.kito_modules.iter().any(|module| module.mandatory));
    }
}
