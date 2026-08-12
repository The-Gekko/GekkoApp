use crate::core::reporter::{Reporter, BOLD, DIM, FG_CYAN, FG_GREEN, FG_RED, FG_YELLOW, RESET};
use crate::core::system::{
    check_arch_linux, configurar_fastfetch, desinstalar_paquetes, instalar_paquetes,
    is_package_installed, is_solus_linux, print_detected_environment, run_shell, run_shell_piped,
};
use crate::environment::SystemEnvironment;
use crate::installer::{InstallPaths, InstallationPlan};
use crate::kito::{ModuleSelection, ReleaseState};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

pub fn install_zsh_starship(reporter: &dyn Reporter) -> bool {
    reporter.header("INSTALANDO ZSH Y TERMINAL BONITA  (by GekkoApp)");
    thread::sleep(Duration::from_millis(500));

    let solus = is_solus_linux();
    if !check_arch_linux() && !solus {
        reporter.err("El preset de terminal solo esta disponible en Arch Linux y Solus.");
        return false;
    }

    let packages: &[&str] = if solus {
        &[
            "kitty",
            "git",
            "zsh",
            "nano",
            "curl",
            "eza",
            "fastfetch",
            "fzf",
            "font-firacode-nerd",
            "starship",
            "zsh-autosuggestions",
            "zsh-syntax-highlighting",
            "zoxide",
        ]
    } else {
        &[
            "kitty",
            "git",
            "zsh",
            "nano",
            "curl",
            "eza",
            "fastfetch",
            "fzf",
            "ttf-jetbrains-mono-nerd",
            "starship",
            "zsh-autosuggestions",
            "zsh-syntax-highlighting",
            "zsh-history-substring-search",
            "zsh-completions",
            "zoxide",
        ]
    };
    if !instalar_paquetes(reporter, packages) {
        reporter.err("Fallo la instalación de dependencias base y plugins de ZSH.");
        return false;
    }

    if !run_shell("fc-cache -fv > /dev/null 2>&1") {
        reporter.warn("Fallo al actualizar caché de fuentes (fc-cache).");
    }

    let zshrc = build_zshrc(solus);

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let zshrc_path = format!("{}/.zshrc", home);

    reporter.info(&format!("Se sobrescribirá el archivo {}", zshrc_path));
    if reporter.confirm("¿Deseas sobrescribir ~/.zshrc con la nueva configuración?") {
        if Path::new(&zshrc_path).exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if let Err(e) =
                std::fs::copy(&zshrc_path, format!("{}.backup_{}", zshrc_path, timestamp))
            {
                reporter.err(&format!("Fallo al respaldar .zshrc: {}", e));
                return false;
            }
        }
        if let Err(e) = std::fs::write(&zshrc_path, zshrc) {
            reporter.err(&format!("Fallo al escribir .zshrc: {}", e));
            return false;
        }
        reporter.ok(".zshrc escrito correctamente.");
    }

    if reporter.confirm("¿Deseas aplicar el preset 'jetpack' de Starship?") {
        reporter.info("Estableciendo preset de Starship...");
        let _ = run_shell("mkdir -p ~/.config");
        let starship_cfg = format!("{}/.config/starship.toml", home);
        if Path::new(&starship_cfg).exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if let Err(e) = std::fs::copy(
                &starship_cfg,
                format!("{}.backup_{}", starship_cfg, timestamp),
            ) {
                reporter.err(&format!("Fallo al respaldar starship.toml: {}", e));
                return false;
            }
        }
        if !run_shell("starship preset jetpack -o ~/.config/starship.toml") {
            reporter.err("Fallo al escribir el preset de Starship.");
            return false;
        }
    }

    if !configurar_fastfetch(reporter) {
        reporter.err("Fallo la configuración de fastfetch.");
        return false;
    }

    if reporter.confirm("¿Deseas cambiar tu shell predeterminada a ZSH?")
        && !run_shell("sudo chsh -s /bin/zsh $USER")
    {
        reporter.err("Fallo al cambiar la shell.");
        return false;
    }

    reporter.ok("¡Instalación de Terminal Finalizada! — by GekkoApp");
    reporter.warn("Reinicia sesión o ejecuta: exec zsh  para aplicar cambios.");
    thread::sleep(Duration::from_secs(2));
    true
}

/// Construye el `~/.zshrc` con las rutas de plugins validas para la distro.
///
/// Arch Linux: zsh-completions + history-substring-search y plugins bajo
/// `/usr/share/zsh/plugins/`. Solus: las rutas difieren
/// (`/usr/share/zsh-autosuggestions/`, fzf en `/usr/share/zsh/site-functions/`
/// y zsh-syntax-highlighting en `site-functions`); history-substring-search y
/// zsh-completions no estan empaquetados en los repos de Solus.
fn build_zshrc(solus: bool) -> String {
    let fpath_line = if solus {
        "fpath=(/usr/share/zsh/site-functions $fpath)"
    } else {
        "fpath=(/usr/share/zsh/plugins/zsh-completions/src $fpath)"
    };
    let plugins = if solus {
        "source /usr/share/zsh-autosuggestions/zsh-autosuggestions.zsh\n\
         source /usr/share/fzf/key-bindings.zsh\n\
         source /usr/share/zsh/site-functions/zsh-syntax-highlighting.zsh"
    } else {
        "source /usr/share/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh\n\
         source /usr/share/zsh/plugins/zsh-history-substring-search/zsh-history-substring-search.zsh\n\
         bindkey '^[[A' history-substring-search-up\n\
         bindkey '^[[B' history-substring-search-down\n\
         source /usr/share/fzf/key-bindings.zsh\n\
         source /usr/share/fzf/completion.zsh\n\
         source /usr/share/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh"
    };
    format!(
        r#"# =====================================
# HISTORIAL
# =====================================

HISTFILE=~/.zsh_history
HISTSIZE=100000
SAVEHIST=100000

setopt APPEND_HISTORY
setopt INC_APPEND_HISTORY
setopt SHARE_HISTORY
setopt HIST_IGNORE_ALL_DUPS
setopt HIST_IGNORE_SPACE
setopt HIST_REDUCE_BLANKS
setopt HIST_VERIFY

# =====================================
# COMPLETION
# =====================================

autoload -Uz compinit
{fpath_line}
compinit -d ~/.zcompdump

zstyle ':completion:*' matcher-list 'm:{{a-z}}={{A-Za-z}}' 'r:|[._-]=* r:|=*'

setopt AUTO_MENU
setopt COMPLETE_IN_WORD
setopt ALWAYS_TO_END

# =====================================
# ALIASES
# =====================================

alias l='eza -l --icons --color=auto --group-directories-first'
alias ls='eza --icons --color=auto --group-directories-first'
alias ll='eza -l --icons --color=auto --group-directories-first'
alias la='eza -la --icons --color=auto --group-directories-first'
alias lt='eza --tree --icons --color=auto --group-directories-first'

# =====================================
# PROMPT
# =====================================

eval "$(starship init zsh)"

# =====================================
# PLUGINS
# =====================================

{plugins}
eval "$(zoxide init zsh)"

# =====================================
# KEYBINDINGS
# =====================================

stty -ixon

bindkey '^Q' kill-whole-line
bindkey '^U' backward-kill-line
bindkey '^K' kill-line
bindkey '^[[1;5C' forward-word
bindkey '^[[1;5D' backward-word

# =====================================
# FASTFETCH SOLO INTERACTIVO (by The-Gekko)
# =====================================

[[ $- == *i* ]] && fastfetch

# ZSH Config by iBlueMoon
"#,
        fpath_line = fpath_line,
        plugins = plugins,
    )
}

pub fn install_hyprland(reporter: &dyn Reporter) -> bool {
    reporter.header("INSTALANDO PRESET HYPRLAND");
    if !check_arch_linux() {
        reporter.err("El preset Hyprland solo esta disponible en Arch Linux y derivadas.");
        return false;
    }
    if !instalar_paquetes(
        reporter,
        &[
            "gedit",
            "electron",
            "gnome-keyring",
            "polkit-gnome",
            "xdg-desktop-portal-gtk",
            "xwayland-satellite",
            "nwg-look",
            "blueman",
            "ibus",
            "unzip",
            "colord",
            "bolt",
            "flatpak",
            "gnome-disk-utility",
            "gvfs-mtp",
            "gvfs-gphoto2",
            "libmtp",
            "nautilus",
            "fuse2",
            "ddcutil",
            "i2c-dev",
        ],
    ) {
        return false;
    }

    if !desinstalar_paquetes(reporter, &["dolphin", "polkit-kde-agent", "wofi"]) {
        reporter.warn("Hubo un problema o se canceló la desinstalación. Abortando instalación.");
        return false;
    }

    reporter.ok("Evaluación de dependencias de Hyprland finalizada.");
    thread::sleep(Duration::from_secs(2));
    true
}

pub fn install_niri(reporter: &dyn Reporter) -> bool {
    reporter.header("INSTALANDO PRESET NIRI");
    if !check_arch_linux() {
        reporter.err("El preset Niri solo esta disponible en Arch Linux y derivadas.");
        return false;
    }
    if !instalar_paquetes(
        reporter,
        &[
            "gedit",
            "xdg-desktop-portal-gnome",
            "gnome-keyring",
            "polkit-gnome",
            "xdg-desktop-portal-gtk",
            "electron",
            "xwayland-satellite",
            "gnome-disk-utility",
            "qt5-wayland",
            "qt6-wayland",
            "nwg-look",
            "flatpak",
            "blueman",
            "gvfs-mtp",
            "gvfs-gphoto2",
            "libmtp",
            "ibus",
            "unzip",
            "colord",
            "bolt",
            "fuse2",
            "ddcutil",
            "i2c-dev",
            "playerctl",
            "gpsd",
            "dconf-editor",
        ],
    ) {
        return false;
    }

    if !desinstalar_paquetes(
        reporter,
        &["mako", "swaybg", "swayidle", "swaylock", "waybar"],
    ) {
        reporter.warn("Hubo un problema o se canceló la desinstalación. Abortando instalación.");
        return false;
    }
    reporter.ok("Evaluación de dependencias de Niri finalizada.");
    thread::sleep(Duration::from_secs(2));
    true
}

pub fn install_gaming(reporter: &dyn Reporter, gpu: &str, vulkan_choice: &str) -> bool {
    let label = match gpu {
        "nvidia" => "INSTALANDO UTILIDADES GAMING  ·  NVIDIA",
        "intel" => "INSTALANDO UTILIDADES GAMING  ·  INTEL",
        "amd" => "INSTALANDO UTILIDADES GAMING  ·  AMD",
        _ => "INSTALANDO UTILIDADES GAMING",
    };
    reporter.header(label);

    let solus = is_solus_linux();
    if !check_arch_linux() && !solus {
        reporter.err("El sistema no es Arch Linux ni Solus. Cancelando.");
        return false;
    }

    if !reporter.confirm(&format!(
        "¿Deseas proceder con la instalación del entorno Gaming para {}?",
        gpu.to_uppercase()
    )) {
        return false;
    }

    if solus {
        return install_gaming_solus(reporter);
    }

    reporter.info("Instalando steam...");
    let steam_cmd = format!(
        "echo -e '{}\\ns' | sudo pacman -S --needed steam",
        vulkan_choice
    );
    if !run_shell(&steam_cmd) {
        reporter.warn("Steam no se pudo instalar correctamente o fue cancelado.");
        return false;
    }

    reporter.info("Instalando dxvk (seleccionando proveedor 1 automáticamente)...");
    if !is_package_installed("dxvk-async-git") && !is_package_installed("dxvk-mingw-git") {
        if !run_shell("echo '1' | sudo pacman -S --needed dxvk 2>&1") {
            reporter.warn("Fallo al instalar dxvk.");
            return false;
        }
    } else {
        reporter.ok("dxvk ya está instalado. Saltando...");
    }

    reporter.info("Instalando dependencias de Proton GE (lib32-gstreamer)...");
    if !instalar_paquetes(reporter, &["lib32-gstreamer", "lib32-gst-plugins-base"]) {
        reporter.warn("Fallo al instalar dependencias de lib32.");
        return false;
    }

    if !instalar_paquetes(
        reporter,
        &[
            "protonplus",
            "spotify",
            "discord",
            "gamemode",
            "gedit",
            "flatpak",
        ],
    ) {
        reporter.warn("Fallo al instalar algunas herramientas gaming.");
        return false;
    }

    reporter.info("Instalando proton-ge-custom-bin...");
    if !is_package_installed("proton-ge-custom-bin") {
        let ok = run_shell("sudo pacman -S --needed --noconfirm proton-ge-custom-bin 2>&1");
        if ok {
            reporter.ok("proton-ge-custom-bin instalado.");
        } else {
            reporter.warn("proton-ge-custom-bin no pudo instalarse. Instálalo manualmente con: sudo pacman -S proton-ge-custom-bin");
            return false;
        }
    } else {
        reporter.ok("proton-ge-custom-bin ya está instalado.");
    }

    reporter.ok("¡Proceso de instalación Gaming terminado!");
    thread::sleep(Duration::from_secs(2));
    true
}

/// Variante del flujo gaming para Solus: usa eopkg, paquetes multilib
/// (`gstreamer-1.0-32bit`, `gstreamer-1.0-plugins-base-32bit`) y omite
/// Proton GE / DXVK / protonplus / spotify, que no existen en los repos
/// de Solus (se deja indicado instalar ProtonUp-Qt por Flatpak).
fn install_gaming_solus(reporter: &dyn Reporter) -> bool {
    reporter.info("Instalando Steam (Solus)...");
    if !instalar_paquetes(reporter, &["steam"]) {
        reporter.warn("Steam no se pudo instalar correctamente o fue cancelado.");
        return false;
    }

    reporter.info("Instalando librerias 32-bit y Flatpak...");
    if !instalar_paquetes(
        reporter,
        &[
            "gstreamer-1.0-32bit",
            "gstreamer-1.0-plugins-base-32bit",
            "flatpak",
        ],
    ) {
        reporter.warn("Fallo al instalar las dependencias 32-bit de Solus.");
        return false;
    }

    reporter.info("Instalando herramientas gaming...");
    if !instalar_paquetes(reporter, &["gamemode", "discord", "gedit", "mangohud"]) {
        reporter.warn("Fallo al instalar algunas herramientas gaming de Solus.");
        return false;
    }

    reporter.info("Solus no empaqueta Proton GE, DXVK ni protonplus en sus repositorios.");
    reporter.info("Instala ProtonUp-Qt (Flatpak) y usalo para gestionar Proton GE.");
    reporter.ok("¡Proceso de instalación Gaming terminado!");
    thread::sleep(Duration::from_secs(2));
    true
}

/// Instala o actualiza Bauh Fork (The-Gekko) desde GitHub Releases.
///
/// El release se publica con un manifiesto firmado (SHA-256) que declara el
/// metodo `python_pipx`: se verifica el artefacto fuente y se instala con
/// pipx (desinstalando antes una instalacion pipx previa que bloquearia la
/// activacion). La GUI pasa `require_confirmation = false` porque
/// la confirmacion ya se solicito en su catalogo.
pub fn install_bauh(
    reporter: &dyn Reporter,
    environment: &SystemEnvironment,
    require_confirmation: bool,
) -> Result<(), String> {
    reporter.header("INSTALANDO TIENDA BAUH FORK (THE-GEKKO)");

    let solus = is_solus_linux();
    if !check_arch_linux() && !solus {
        return Err("La tienda Bauh Fork solo esta disponible en Arch Linux y Solus.".to_owned());
    }

    let target = environment
        .target()
        .ok_or_else(|| "No existe un target de release para esta arquitectura.".to_owned())?;

    if is_package_installed("bauh") {
        if solus {
            return Err("Bauh no se distribuye en los repos de Solus; no hay conflicto.".to_owned());
        }
        if require_confirmation
            && !reporter.confirm(
                "¿Deseas desinstalar el bauh original de pacman para evitar conflictos con el fork?",
            )
        {
            return Err("Instalacion cancelada: se mantuvo el bauh de pacman.".to_owned());
        }
        if !desinstalar_paquetes(reporter, &["bauh"]) {
            return Err("No se pudo desinstalar el paquete bauh oficial.".to_owned());
        }
    }

    let pipx = if solus { "pipx" } else { "python-pipx" };
    if !is_package_installed(pipx) && !instalar_paquetes(reporter, &[pipx]) {
        return Err("No se pudieron instalar las dependencias base (pipx).".to_owned());
    }

    reporter.step("Verificando el release mas reciente de Bauh Fork...");
    let plan = crate::core::catalog::resolve_bauh_plan(target)?;
    reporter.ok(&format!(
        "Release de Bauh Fork {} encontrado y manifiesto verificado.",
        plan.releases[0].manifest.product.version
    ));

    let paths = InstallPaths::detect()?;
    reporter.step("Descargando y verificando el artefacto firmado...");
    plan.prefetch(&paths)?;
    reporter.step("Instalando Bauh Fork con pipx...");
    let state = plan.install(&paths)?;

    let version = state
        .modules
        .get(crate::core::catalog::BAUH_PRODUCT_ID)
        .map(|module| module.version.as_str())
        .unwrap_or("desconocida");
    reporter.ok(&format!(
        "¡Bauh Fork (The-Gekko) {version} instalado correctamente!"
    ));
    println!(
        "      {}Ejecuta la tienda con: {}/bauh{}",
        DIM,
        paths.bin_home.display(),
        RESET
    );
    thread::sleep(Duration::from_secs(2));
    Ok(())
}

/// Instala o actualiza GekkoApp a si misma desde GitHub Releases.
///
/// Usa el mismo motor que el Bauh Fork: se resuelve el release mas reciente,
/// se verifica el manifiesto firmado (SHA-256, contrato `kitotsu.release-artifact`
/// 1.0) y se activa con el layout nativo de symlinks. Como GekkoApp se instalo
/// antes con `scripts/install.sh` (archivos regulares sin registrar), se
/// adoptan esas rutas legacy antes de activar para que el motor pueda
/// gestionarlas. No requiere sudo: solo escribe en los XDG paths del usuario.
pub fn install_gekkoapp(
    reporter: &dyn Reporter,
    environment: &SystemEnvironment,
    require_confirmation: bool,
) -> Result<(), String> {
    reporter.header("INSTALANDO GEKKOAPP (THE-GEKKO)");

    if require_confirmation
        && !reporter.confirm("¿Deseas actualizar GekkoApp a la ultima version publicada?")
    {
        return Err("Actualizacion cancelada.".to_owned());
    }

    let target = environment
        .target()
        .ok_or_else(|| "No existe un target de release para esta arquitectura.".to_owned())?;

    reporter.step("Verificando el release mas reciente de GekkoApp...");
    let plan = crate::core::catalog::resolve_gekkoapp_plan(target)?;
    reporter.ok(&format!(
        "Release de GekkoApp {} encontrado y manifiesto verificado.",
        plan.releases[0].manifest.product.version
    ));

    let paths = InstallPaths::detect()?;
    reporter.step("Descargando y verificando el artefacto firmado...");
    plan.prefetch(&paths)?;
    reporter.step("Adoptando los binarios e integracion previos...");
    crate::installer::adopt_release_destinations(&paths, &plan)?;
    let _ = fs::remove_file(
        paths
            .data_home
            .join("applications")
            .join("gekkoapp-control-center.desktop"),
    );
    reporter.step("Actualizando GekkoApp...");
    let state = plan.install(&paths)?;

    let version = state
        .modules
        .get(crate::core::catalog::GEKKOAPP_PRODUCT_ID)
        .map(|module| module.version.as_str())
        .unwrap_or("desconocida");
    reporter.ok(&format!("¡GekkoApp {version} actualizado correctamente!"));
    reporter.warn("Reinicia GekkoApp para ejecutar la nueva version.");
    thread::sleep(Duration::from_secs(2));
    Ok(())
}
///
/// El proyecto no publica releases firmados todavia, asi que se clona HEAD,
/// se instalan las dependencias de sistema con pacman y se ejecuta su propio
/// `install.sh --no-deps` (mismo instalador que usa el autor: copia la app a
/// XDG, crea el launcher, el desktop entry, el icono y el metainfo). La
/// revision instalada se registra en el estado de GekkoApp.
pub fn install_gekko_adb(reporter: &dyn Reporter) -> Result<(), String> {
    reporter.header("INSTALANDO GEKKO ADB STUDIO (THE-GEKKO)");

    let solus = is_solus_linux();
    if !check_arch_linux() && !solus {
        return Err("Gekko ADB Studio solo esta disponible en Arch Linux y Solus.".to_owned());
    }

    const DEPS_ARCH: &[&str] = &[
        "git",
        "python",
        "python-gobject",
        "gtk3",
        "gtk4",
        "android-tools",
        "scrcpy",
        "glib2",
        "xdg-utils",
    ];
    let deps: &[&str] = if solus {
        &[
            "git",
            "python-3",
            "python-gobject",
            "gtk-3",
            "gtk-4",
            "android-tools",
            "scrcpy",
            "glib-2",
            "xdg-utils",
        ]
    } else {
        DEPS_ARCH
    };
    if !instalar_paquetes(reporter, deps) {
        return Err("No se pudieron instalar las dependencias de Gekko ADB Studio.".to_owned());
    }

    let paths = InstallPaths::detect()?;
    let source_dir = paths
        .cache_home
        .join(crate::core::catalog::GEKKO_ADB_PRODUCT_ID);
    fs::create_dir_all(&source_dir)
        .map_err(|error| format!("No se pudo preparar {}: {error}", source_dir.display()))?;
    let quoted_dir = sh_quote(&source_dir);

    reporter.step("Descargando el codigo fuente desde GitHub...");
    let repo_url = format!(
        "https://github.com/{}.git",
        crate::core::catalog::GEKKO_ADB_REPOSITORY
    );
    if source_dir.join(".git").exists() {
        if !run_shell(&format!(
            "git -C {quoted_dir} fetch --depth 1 origin main && git -C {quoted_dir} reset --hard origin/main"
        )) {
            return Err("No se pudo actualizar el codigo fuente de Gekko ADB Studio.".to_owned());
        }
    } else if !run_shell(&format!("git clone --depth 1 {repo_url} {quoted_dir}")) {
        return Err("No se pudo descargar el codigo fuente de Gekko ADB Studio.".to_owned());
    }

    let version = run_shell_piped(&format!("git -C {quoted_dir} rev-parse --short HEAD")).1;
    let version = version.trim().to_string();
    if version.is_empty() {
        return Err("No se pudo determinar la revision de Gekko ADB Studio.".to_owned());
    }

    reporter.step("Ejecutando el instalador de Gekko ADB Studio...");
    let installer = source_dir.join("install.sh");
    if !run_shell(&format!("{} --no-deps --assume-yes", sh_quote(&installer))) {
        return Err("El instalador de Gekko ADB Studio fallo.".to_owned());
    }

    let mut entrypoints = BTreeMap::new();
    entrypoints.insert(
        "gekko-adb".to_string(),
        paths.bin_home.join("gekko-adb").display().to_string(),
    );
    crate::installer::record_source_module(
        &paths,
        crate::core::catalog::GEKKO_ADB_PRODUCT_ID,
        &version,
        &source_dir.display().to_string(),
        entrypoints,
    )?;

    reporter.ok(&format!(
        "¡Gekko ADB Studio {version} instalado correctamente!"
    ));
    println!(
        "      {}Ejecuta la suite con: {}/gekko-adb{}",
        DIM,
        paths.bin_home.display(),
        RESET
    );
    thread::sleep(Duration::from_secs(2));
    Ok(())
}

/// Escapa una ruta para usarla como argumento unico de una shell.
fn sh_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

fn prompt_with_default(reporter: &dyn Reporter, label: &str, current: &str) -> String {
    reporter.prompt(label, current)
}

fn confirm_or_override_environment(
    reporter: &dyn Reporter,
    mut environment: SystemEnvironment,
) -> Option<SystemEnvironment> {
    loop {
        print_detected_environment(reporter, &environment);
        println!();
        println!("  {}[1]{} Usar deteccion", FG_CYAN, RESET);
        println!("  {}[2]{} Corregir deteccion manualmente", FG_CYAN, RESET);
        println!("  {}[0]{} Cancelar", FG_RED, RESET);
        print!("  {}Opcion:{} ", BOLD, RESET);
        let _ = io::stdout().flush();
        match reporter.read_line().as_str() {
            "1" => return Some(environment),
            "2" => {
                environment.distro_id =
                    prompt_with_default(reporter, "ID de distribucion", &environment.distro_id);
                environment.distro_name = environment.distro_id.clone();
                environment.session = prompt_with_default(reporter, "Sesion", &environment.session);
                environment.desktop =
                    prompt_with_default(reporter, "Escritorio", &environment.desktop);
                environment.refresh_compatibility();
                println!();
            }
            "0" => return None,
            _ => reporter.warn("Selecciona 1, 2 o 0."),
        }
    }
}

fn select_kito_modules(reporter: &dyn Reporter) -> Option<ModuleSelection> {
    let mut selection = ModuleSelection::default();
    loop {
        reporter.clear_screen();
        reporter.header("MODULOS DEL ENTORNO KITO");
        println!("  {}Obligatorios{}", BOLD, RESET);
        println!("  {}[✓]{} KiUI", FG_GREEN, RESET);
        println!("  {}[✓]{} Kitsune Compositor", FG_GREEN, RESET);
        println!();
        println!("  {}Selecciona uno o varios modulos{}", BOLD, RESET);
        println!(
            "  [{}] [1] Kitowall       Wallpapers estaticos",
            if selection.kitowall { "x" } else { " " }
        );
        println!(
            "  [{}] [2] Kilivepaper    Live wallpapers",
            if selection.kilivepaper { "x" } else { " " }
        );
        println!(
            "  [{}] [3] KiSDDM          Pantalla de inicio SDDM",
            if selection.kisddm { "x" } else { " " }
        );
        println!(
            "  {}[--] [4] Kitsune        Espectro de audio  [PROXIMAMENTE]{}",
            DIM, RESET
        );
        println!();
        println!("  {}[5]{} Continuar", FG_CYAN, RESET);
        println!("  {}[0]{} Cancelar", FG_RED, RESET);
        print!("  {}Opcion:{} ", BOLD, RESET);
        let _ = io::stdout().flush();
        match reporter.read_line().as_str() {
            "1" => selection.kitowall = !selection.kitowall,
            "2" => selection.kilivepaper = !selection.kilivepaper,
            "3" => selection.kisddm = !selection.kisddm,
            "4" => {
                reporter.warn("Kitsune estara disponible proximamente.");
                thread::sleep(Duration::from_secs(1));
            }
            "5" if selection.has_product() => return Some(selection),
            "5" => {
                reporter.warn("Selecciona al menos Kitowall, Kilivepaper o KiSDDM.");
                thread::sleep(Duration::from_secs(2));
            }
            "0" => return None,
            _ => {
                reporter.warn("Opcion no valida.");
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

/// Non-interactive install of a concrete Kito module selection.
///
/// Used by the CLI after module selection and by the GUI from its catalog.
/// Returns the number of active modules on success.
pub fn install_kito_plan(
    reporter: &dyn Reporter,
    environment: SystemEnvironment,
    selection: ModuleSelection,
    require_confirmation: bool,
) -> Result<usize, String> {
    if !environment.compatibility.supported {
        return Err(
            "La instalacion se bloqueo para evitar una configuracion incompatible.".to_owned(),
        );
    }
    let target = environment
        .target()
        .ok_or_else(|| "No existe un target de release para esta arquitectura.".to_owned())?;
    let plan_components = selection.plan();

    reporter.header("VERIFICANDO RELEASES DE KITO");
    reporter.info("Consultando releases estables publicados en GitHub...");
    let statuses = crate::kito::resolve_releases(&plan_components, target);
    for status in &statuses {
        match &status.state {
            ReleaseState::Available {
                version,
                tag,
                manifest_url,
                ..
            } => {
                reporter.ok(&format!(
                    "{} {} ({})",
                    status.component.label(),
                    version,
                    tag
                ));
                println!("      {}{}{}", DIM, manifest_url, RESET);
            }
            ReleaseState::Unavailable(reason) => {
                reporter.err(&format!("{}: {}", status.component.label(), reason));
            }
        }
    }

    if !crate::kito::all_available(&statuses) {
        return Err("No se modifico el sistema: faltan releases obligatorios.".to_owned());
    }

    reporter.step("Descargando y validando manifiestos...");
    let installation = InstallationPlan::prepare(&statuses, target)?;
    let packages = installation.required_arch_packages();
    reporter.ok("Preflight completo: manifests, dependencias y artefactos son coherentes.");
    println!();
    println!("  {}Componentes:{}", BOLD, RESET);
    for release in &installation.releases {
        println!(
            "    - {} {}",
            release.component_label, release.manifest.product.version
        );
    }
    println!("  {}Dependencias del sistema:{}", BOLD, RESET);
    if packages.is_empty() {
        println!("    - Ninguna adicional");
    } else {
        println!("    - {}", packages.join(", "));
    }

    if require_confirmation {
        print!("  {}Instalar este plan?{} [s/N]: ", FG_YELLOW, RESET);
        let _ = io::stdout().flush();
        if !matches!(
            reporter.read_line().to_ascii_lowercase().as_str(),
            "s" | "si"
        ) {
            return Err("Instalacion cancelada sin modificar el sistema.".to_owned());
        }
    }

    let paths = InstallPaths::detect()?;
    reporter.step("Descargando y verificando todos los artefactos antes de modificar paquetes...");
    installation.prefetch(&paths)?;
    if !instalar_paquetes(reporter, &packages) {
        return Err("Se aborto antes de instalar los artefactos Kito.".to_owned());
    }
    reporter.step("Descargando, verificando e instalando artefactos Kito...");
    let state = installation.install(&paths)?;
    reporter.ok(&format!(
        "Entorno Kito instalado: {} componentes activos.",
        state.modules.len()
    ));
    println!(
        "      {}Estado: {}{}",
        DIM,
        paths.state_file().display(),
        RESET
    );
    println!(
        "      {}Ejecuta KiUI con: {}/kiui{}",
        DIM,
        paths.bin_home.display(),
        RESET
    );
    Ok(state.modules.len())
}

pub fn install_kito_environment(reporter: &dyn Reporter) {
    reporter.clear_screen();
    reporter.header("INSTALAR ENTORNO KITO");
    let Some(environment) = confirm_or_override_environment(reporter, SystemEnvironment::detect())
    else {
        return;
    };
    if !environment.compatibility.supported {
        reporter.err("La instalacion se bloqueo para evitar una configuracion incompatible.");
        reporter
            .info("La deteccion manual corrige falsos positivos; no habilita soporte inexistente.");
        return;
    }
    let Some(selection) = select_kito_modules(reporter) else {
        return;
    };
    match install_kito_plan(reporter, environment, selection, true) {
        Ok(_) => {}
        Err(error) => {
            reporter.err(&format!(
                "La instalacion de Kito no pudo completarse: {error}"
            ));
            reporter.info("Los releases versionados no activados pueden permanecer en cache.");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Desinstaladores
// ─────────────────────────────────────────────────────────────────────────────

/// Desinstala Bauh Fork (The-Gekko).
pub fn uninstall_bauh(reporter: &dyn Reporter) -> Result<(), String> {
    reporter.header("DESINSTALANDO TIENDA BAUH FORK");
    let paths = InstallPaths::detect()?;
    let launcher = paths.bin_home.join("bauh");
    if launcher.exists() || is_package_installed("bauh-fork-the-gekko") {
        reporter.step("Ejecutando pipx uninstall...");
        let _ = run_shell("pipx uninstall bauh-fork-the-gekko");
        let _ = run_shell("pipx uninstall bauh");
    }
    crate::installer::uninstall_registered_module(crate::core::catalog::BAUH_PRODUCT_ID)?;
    reporter.ok("Bauh Fork desinstalado correctamente.");
    thread::sleep(Duration::from_secs(1));
    Ok(())
}

/// Desinstala Gekko ADB Studio.
pub fn uninstall_gekko_adb(reporter: &dyn Reporter) -> Result<(), String> {
    reporter.header("DESINSTALANDO GEKKO ADB STUDIO");
    let paths = InstallPaths::detect()?;
    let launcher = paths.bin_home.join("gekko-adb");
    if launcher.exists() || fs::symlink_metadata(&launcher).is_ok() {
        let _ = fs::remove_file(&launcher);
    }
    let app_desktop = paths.data_home.join("applications/com.gekko.adb.desktop");
    if app_desktop.exists() {
        let _ = fs::remove_file(&app_desktop);
    }
    let legacy_desktop = paths.data_home.join("applications/org.thegekko.gekko_adb.desktop");
    if legacy_desktop.exists() {
        let _ = fs::remove_file(&legacy_desktop);
    }
    let icon_file = paths.data_home.join("icons/hicolor/512x512/apps/gekko-adb.png");
    if icon_file.exists() {
        let _ = fs::remove_file(&icon_file);
    }
    let gekko_adb_app_data = paths.data_home.join("gekko-adb");
    if gekko_adb_app_data.exists() {
        let _ = fs::remove_dir_all(&gekko_adb_app_data);
    }
    crate::installer::uninstall_registered_module(crate::core::catalog::GEKKO_ADB_PRODUCT_ID)?;
    reporter.ok("Gekko ADB Studio desinstalado correctamente.");
    thread::sleep(Duration::from_secs(1));
    Ok(())
}


/// Desinstala los modulos de Kito registrados.
pub fn uninstall_kito_environment(reporter: &dyn Reporter) -> Result<(), String> {
    reporter.header("DESINSTALANDO ENTORNO KITO");
    for id in ["kitsune-compositor", "kiui", "kitowall", "kilivepaper", "kisddm"] {
        let _ = crate::installer::uninstall_registered_module(id);
    }
    reporter.ok("Entorno Kito desinstalado correctamente.");
    thread::sleep(Duration::from_secs(1));
    Ok(())
}

/// Desinstala Terminal Bonita (.zshrc).
pub fn uninstall_zsh_starship(reporter: &dyn Reporter) -> bool {
    reporter.header("DESINSTALANDO TERMINAL BONITA");
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let zshrc_path = format!("{}/.zshrc", home);
    if std::path::Path::new(&zshrc_path).exists() {
        let _ = std::fs::remove_file(&zshrc_path);
        reporter.ok("Se ha eliminado el .zshrc configurado por GekkoApp.");
    }
    true
}

/// Desinstala preset Hyprland.
pub fn uninstall_hyprland(reporter: &dyn Reporter) -> bool {
    reporter.header("DESINSTALANDO PRESET HYPRLAND");
    reporter.info("Eliminando dependencias especificas del preset...");
    desinstalar_paquetes(reporter, &["wofi", "dolphin"])
}

/// Desinstala preset Niri.
pub fn uninstall_niri(reporter: &dyn Reporter) -> bool {
    reporter.header("DESINSTALANDO PRESET NIRI");
    reporter.info("Eliminando dependencias especificas del preset...");
    desinstalar_paquetes(reporter, &["niri", "mako"])
}

/// Desinstala preset Gaming.
pub fn uninstall_gaming(reporter: &dyn Reporter) -> bool {
    reporter.header("DESINSTALANDO GAMING SETUP");
    let packages = if is_solus_linux() {
        vec!["gamemode", "mangohud"]
    } else {
        vec!["gamemode", "mangohud", "protonup-qt"]
    };
    desinstalar_paquetes(reporter, &packages)
}

