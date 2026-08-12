//! Interfaz de escritorio Tauri v2 (Control Center).
//!
//! Expone el catalogo y la instalacion de componentes (Kito y Bauh Fork) al
//! frontend `ui/` a traves de comandos. El progreso de los flujos se emite
//! como eventos `install://event` mediante un [`GuiReporter`].
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
    distro_name: String,
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
            installed_version: installed.get(component.id()).cloned(),
        })
        .collect();

    let kito_modules = all_components()
        .into_iter()
        .filter_map(|component| match component {
            CatalogComponent::Kito(component) => Some(ModuleView {
                product_id: component.product_id(),
                label: component.label(),
                mandatory: matches!(component, ComponentId::Compositor | ComponentId::Kiui),
                installed_version: installed.get(component.product_id()).cloned(),
            }),
            CatalogComponent::BauhFork => None,
        })
        .collect();

    let target = environment.target();

    CatalogView {
        distro_name: environment.distro_name,
        session: environment.session,
        desktop: environment.desktop,
        target,
        compatible: environment.compatibility.supported,
        items,
        kito_modules,
    }
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

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            catalog_state,
            install_kito,
            install_bauh
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

        assert_eq!(view.items.len(), 6);
        assert!(view
            .items
            .iter()
            .any(|item| item.id == "kitsune-compositor"));
        assert!(view
            .items
            .iter()
            .any(|item| item.id == "bauh-fork-the-gekko"));
        assert_eq!(view.kito_modules.len(), 5);
        assert!(view.kito_modules.iter().any(|module| module.mandatory));
    }
}
