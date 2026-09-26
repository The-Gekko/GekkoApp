//! Interfaz de escritorio Tauri v2 (Control Center).
//!
//! Expone el catalogo y la instalacion de componentes (Kito, Bauh Fork, Gekko
//! ADB Studio, GekkoApp) y el alta de Chaotic AUR al frontend `ui/` a traves de
//! comandos. El
//! progreso de los flujos se emite como eventos `install://event` mediante un
//! [`GuiReporter`]. El comando `check_updates` alimenta la campana de
//! actualizaciones del Control Center.
//!
//! Elevacion de privilegios: la GUI no tiene TTY, asi que la contrasena de
//! sudo se entrega a un helper `askpass` temporal (`SUDO_ASKPASS` +
//! `GEKKOAPP_ASKPASS`) creado con permisos 0600/0700 y eliminado al terminar.
use crate::core::catalog::{all_components, CatalogComponent, GEKKO_ADB_BRANCH};
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
    /// Solo GekkoApp: hay una version instalada mas nueva que la que se esta
    /// ejecutando (se auto-actualizo y falta reiniciar).
    restart_pending: bool,
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
    // Gekko ADB se registra en el estado con el hash git del clon, no con su
    // version: se consulta primero al lanzador para mostrar la real y el estado
    // queda como respaldo. GekkoApp se compara con el binario en ejecucion. Para
    // el resto el estado guarda la version del release.
    if !matches!(
        component,
        CatalogComponent::GekkoAdb | CatalogComponent::GekkoApp
    ) {
        if let Some(version) = installed.get(component.id()) {
            return Some(version.clone());
        }
    }

    let Ok(paths) = InstallPaths::detect() else {
        return None;
    };

    match component {
        CatalogComponent::BauhFork => {
            // El fork publica sus ejecutables como `gekko-bauh*`; `bauh` es el
            // nombre que usaban versiones anteriores y sigue reconociendose para
            // no dar un falso negativo en instalaciones antiguas.
            let mut nombres = vec![crate::core::catalog::BAUH_LAUNCHER];
            nombres.extend_from_slice(crate::core::catalog::BAUH_LEGACY_LAUNCHERS);
            for nombre in nombres {
                let launcher = paths.bin_home.join(nombre);
                // `bash -c` no siempre hereda ~/.local/bin en el PATH, asi que
                // se invoca la ruta absoluta cuando el lanzador esta ahi.
                let cmd = if launcher.exists() {
                    crate::core::system::sh_quote(&launcher)
                } else if is_binary_in_path(nombre) {
                    nombre.to_string()
                } else {
                    continue;
                };
                let (_, version_output) =
                    crate::core::system::run_shell_piped(&format!("{cmd} --version 2>/dev/null"));
                let salida = version_output.trim();
                let version = salida
                    .strip_prefix("gekko-bauh ")
                    .or_else(|| salida.strip_prefix("bauh "))
                    .unwrap_or(salida)
                    .trim();
                if !version.is_empty() {
                    return Some(version.to_string());
                }
                return Some("instalado".to_string());
            }
            None
        }
        CatalogComponent::GekkoAdb => {
            // El install.sh de Gekko ADB no honra XDG_BIN_HOME: mismo criterio
            // que al instalar y desinstalar.
            let launcher = crate::core::flow::gekko_adb_launcher_path(&paths);
            let app_dir = paths.data_home.join("gekko-adb/app");
            if !launcher.exists() && !app_dir.exists() && !is_binary_in_path("gekko-adb") {
                return None;
            }
            // La version real la imprime el propio lanzador (`Gekko ADB Studio
            // 2.1.0`). Se invoca por ruta absoluta cuando esta en ~/.local/bin,
            // porque `bash -c` no siempre lo tiene en el PATH.
            let cmd = if launcher.exists() {
                Some(crate::core::system::sh_quote(&launcher))
            } else if is_binary_in_path("gekko-adb") {
                Some("gekko-adb".to_string())
            } else {
                None
            };
            if let Some(cmd) = cmd {
                let (ok, salida) =
                    crate::core::system::run_shell_piped(&format!("{cmd} --version 2>/dev/null"));
                let salida = salida.trim();
                let version = salida
                    .strip_prefix("Gekko ADB Studio ")
                    .unwrap_or(salida)
                    .trim();
                if ok && !version.is_empty() && !version.contains('\n') {
                    return Some(version.to_string());
                }
            }
            // Respaldo: la revision que GekkoApp registro al instalar.
            if let Some(version) = installed.get(component.id()) {
                return Some(version.clone());
            }
            // El instalador de Gekko ADB **copia** los archivos a `app_dir`, no
            // clona: ahi nunca hay un `.git` que consultar. La unica revision
            // disponible es la del checkout que mantiene GekkoApp en su cache,
            // y solo sirve si su codigo coincide byte a byte con lo instalado:
            // ese clon puede haber quedado de un intento fallido o de otra
            // revision. Si no coincide se dice "instalado" sin inventar nada.
            let checkout = paths
                .cache_home
                .join(crate::core::catalog::GEKKO_ADB_PRODUCT_ID);
            let mismo_codigo = matches!(
                (
                    std::fs::read(app_dir.join("gekko_adb_core.py")),
                    std::fs::read(checkout.join("gekko_adb_core.py")),
                ),
                (Ok(instalado), Ok(clonado)) if instalado == clonado
            );
            if mismo_codigo && checkout.join(".git").exists() {
                let (_, rev) = crate::core::system::run_shell_piped(&format!(
                    "git -C {} rev-parse --short HEAD 2>/dev/null",
                    crate::core::system::sh_quote(&checkout)
                ));
                let rev = rev.trim();
                if !rev.is_empty() {
                    return Some(rev.to_string());
                }
            }
            Some("instalado".to_string())
        }

        CatalogComponent::GekkoApp => Some(
            registered_gekkoapp_newer_than_running(installed)
                .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
        ),
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

/// Version de GekkoApp registrada en el estado si es mas nueva que la que se
/// esta ejecutando: la auto-actualizacion la registra, pero este proceso sigue
/// siendo el anterior hasta reiniciar. Con un binario compilado desde el clon
/// puede pasar lo contrario (el estado guarda un release anterior), y entonces
/// cuenta el binario en ejecucion.
fn registered_gekkoapp_newer_than_running(installed: &BTreeMap<String, String>) -> Option<String> {
    let registered = installed.get(CatalogComponent::GekkoApp.id())?;
    let newer = crate::installer::compare_versions(registered, env!("CARGO_PKG_VERSION"))
        .is_ok_and(|ordering| ordering == std::cmp::Ordering::Greater);
    newer.then(|| registered.clone())
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
            restart_pending: component == CatalogComponent::GekkoApp
                && registered_gekkoapp_newer_than_running(&installed).is_some(),
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

/// Consulta la ultima version publicada de cada componente y la compara con la
/// instalada. Kito, Bauh Fork y el propio GekkoApp se comparan con su ultimo
/// release; Gekko ADB Studio, que no publica releases, con el ultimo commit de
/// la rama que instala GekkoApp. Todo se pregunta a GitHub en el momento: un
/// componente nuevo se detecta sin publicar otra version de GekkoApp.
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
            let installed_version = installed_version_of(&installed, component);
            if matches!(component, CatalogComponent::GekkoAdb) {
                if let Some(update) = gekko_adb_update(&installed, installed_version.is_some()) {
                    updates.push(update);
                }
                continue;
            }
            let latest = crate::core::github::resolve_latest_release(
                component.repository(),
                component.id(),
                target,
            )
            .ok()
            .map(|(tag, _, _)| tag.trim_start_matches('v').to_string());
            let update_available = match (&installed_version, &latest) {
                (Some(current), Some(newer)) => {
                    match crate::installer::compare_versions(newer, current) {
                        Ok(ordering) => ordering == std::cmp::Ordering::Greater,
                        // Alguna de las dos no es comparable numericamente (por
                        // ejemplo el marcador "instalado" de una instalacion que
                        // GekkoApp no registro). Se avisa si difieren, en vez de
                        // afirmar en silencio que todo esta al dia.
                        Err(_) => current != newer,
                    }
                }
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

/// Actualizacion de Gekko ADB Studio: revision registrada al instalar frente al
/// ultimo commit de su rama. Sin revision registrada (lo instalo su propio
/// `install.sh`) no hay con que comparar y no se informa nada.
fn gekko_adb_update(
    installed: &BTreeMap<String, String>,
    is_installed: bool,
) -> Option<UpdateInfo> {
    let component = CatalogComponent::GekkoAdb;
    let recorded = installed.get(component.id()).filter(|_| is_installed)?;
    let head =
        crate::core::github::resolve_branch_head(component.repository(), GEKKO_ADB_BRANCH).ok();
    let update_available = head
        .as_deref()
        .and_then(|head| crate::core::github::same_commit(recorded, head))
        .is_some_and(|same| !same);
    Some(UpdateInfo {
        id: component.id(),
        label: component.label(),
        installed: Some(recorded.clone()),
        // Misma longitud que la revision registrada, para que se lean igual.
        latest: head.map(|head| head[..recorded.len().clamp(7, head.len())].to_string()),
        update_available,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
//  Reinicio tras la auto-actualizacion
// ─────────────────────────────────────────────────────────────────────────────

/// Abre la version de GekkoApp recien instalada y cierra esta.
///
/// No sirve `AppHandle::restart`: relanza el ejecutable en uso, que es el de
/// la version anterior. Se lanza el `gekkoapp-gui` que registro la
/// actualizacion (o el symlink de `~/.local/bin`), que ya apunta a la nueva.
#[tauri::command]
fn restart_gekkoapp(app: AppHandle) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let paths = InstallPaths::detect()?;
    let registered = std::fs::read_to_string(paths.state_file())
        .ok()
        .and_then(|text| serde_json::from_str::<crate::installer::InstallationState>(&text).ok())
        .and_then(|mut state| {
            state
                .modules
                .remove(CatalogComponent::GekkoApp.id())?
                .entrypoints
                .remove("gekkoapp-gui")
        })
        .map(PathBuf::from);
    let launcher = registered
        .filter(|path| path.exists())
        .unwrap_or_else(|| paths.bin_home.join("gekkoapp-gui"));
    if !launcher.exists() {
        return Err(format!(
            "No se encontro {}: cierra y abre GekkoApp a mano.",
            launcher.display()
        ));
    }
    // Grupo de procesos propio: la nueva ventana no depende de esta.
    Command::new(&launcher)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|error| format!("No se pudo abrir {}: {error}", launcher.display()))?;
    app.exit(0);
    Ok(())
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
        return Err(format!(
            "El entorno no es compatible: {}",
            if environment.compatibility.reasons.is_empty() {
                "revisa la deteccion del sistema.".to_string()
            } else {
                environment.compatibility.reasons.join("; ")
            }
        ));
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
            install_chaotic_aur,
            uninstall_kito,
            uninstall_bauh,
            uninstall_gekko_adb,
            restart_gekkoapp,
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

    fn registered_gekkoapp(version: &str) -> BTreeMap<String, String> {
        BTreeMap::from([(
            CatalogComponent::GekkoApp.id().to_string(),
            version.to_string(),
        )])
    }

    #[test]
    fn self_update_counts_as_installed_until_restart() {
        // Se auto-actualizo a una version mas nueva que la que se ejecuta: la
        // campana no debe volver a ofrecerla y el catalogo pide reiniciar.
        let installed = registered_gekkoapp("99.0.0");
        assert_eq!(
            registered_gekkoapp_newer_than_running(&installed).as_deref(),
            Some("99.0.0")
        );
        assert_eq!(
            installed_version_of(&installed, CatalogComponent::GekkoApp).as_deref(),
            Some("99.0.0")
        );
    }

    #[test]
    fn running_binary_wins_over_an_older_or_equal_registration() {
        for registered in ["0.1.0", env!("CARGO_PKG_VERSION"), "no-es-version"] {
            let installed = registered_gekkoapp(registered);
            assert_eq!(registered_gekkoapp_newer_than_running(&installed), None);
            assert_eq!(
                installed_version_of(&installed, CatalogComponent::GekkoApp).as_deref(),
                Some(env!("CARGO_PKG_VERSION"))
            );
        }
        assert_eq!(
            registered_gekkoapp_newer_than_running(&BTreeMap::new()),
            None
        );
    }

    #[test]
    fn gekko_adb_without_a_recorded_revision_reports_nothing() {
        // Sin revision registrada (o sin instalar) no hay con que comparar y no
        // se consulta a GitHub.
        assert!(gekko_adb_update(&BTreeMap::new(), true).is_none());
        let installed = BTreeMap::from([(
            CatalogComponent::GekkoAdb.id().to_string(),
            "4c3f9bf".to_string(),
        )]);
        assert!(gekko_adb_update(&installed, false).is_none());
    }

    #[test]
    #[ignore]
    fn gekko_adb_update_compares_with_the_published_branch_head() {
        // Requiere red. Un commit viejo de main (795828f, anterior a 4c3f9bf)
        // tiene que salir como desactualizado, y el HEAD actual como al dia.
        let old = BTreeMap::from([(
            CatalogComponent::GekkoAdb.id().to_string(),
            "795828f".to_string(),
        )]);
        let update = gekko_adb_update(&old, true).expect("hay revision registrada");
        assert!(update.update_available, "latest = {:?}", update.latest);
        let head = update.latest.expect("GitHub respondio");
        assert_eq!(head.len(), 7);

        let current = BTreeMap::from([(CatalogComponent::GekkoAdb.id().to_string(), head)]);
        let update = gekko_adb_update(&current, true).expect("hay revision registrada");
        assert!(!update.update_available);
    }
}
