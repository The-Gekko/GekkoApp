use crate::core::reporter::{DIM, FG_YELLOW, RESET};
use crate::core::Reporter;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run_shell(cmd: &str) -> bool {
    Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn run_shell_piped(cmd: &str) -> (bool, String) {
    let out = Command::new("bash").arg("-c").arg(cmd).output();
    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            (o.status.success(), stdout)
        }
        Err(_) => (false, String::new()),
    }
}

pub fn sh_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

pub fn is_package_installed(pkg: &str) -> bool {
    if check_arch_linux() {
        run_shell_piped(&format!("pacman -Qq '{pkg}' 2>/dev/null")).0
    } else if is_solus_linux() {
        run_shell_piped(&format!(
            "eopkg list-installed 2>/dev/null | awk '{{print $1}}' | grep -qx '{}'",
            pkg
        ))
        .0
    } else {
        false
    }
}

/// Prefijo de sudo para los comandos privilegiados.
///
/// La GUI establece `GEKKOAPP_ASKPASS` (y `SUDO_ASKPASS`) apuntando a un helper
/// askpass temporal; en ese caso se usa `sudo -A`. En el CLI el prefijo se queda
/// en `sudo` para conservar el prompt de la terminal.
///
/// El `-A` no es un capricho: sudo solo recurre a `SUDO_ASKPASS` por su cuenta
/// cuando *no hay terminal*, y el Control Center lanzado desde una terminal
/// (`./GekkoApp.sh`) hereda esa TTY. Sin `-A`, sudo pediria la contrasena en un
/// terminal que el usuario de la GUI no esta mirando y la operacion pareceria
/// colgada.
pub fn sudo_prefix() -> &'static str {
    if std::env::var_os("GEKKOAPP_ASKPASS").is_some() {
        "sudo -A"
    } else {
        "sudo"
    }
}

/// Construye un `Command` de `sudo` con el modo askpass cuando corresponde.
///
/// Es el equivalente de [`sudo_prefix`] para las invocaciones que no pasan por
/// una shell; misma regla y mismo motivo.
pub fn sudo_command() -> Command {
    let mut command = Command::new("sudo");
    if std::env::var_os("GEKKOAPP_ASKPASS").is_some() {
        command.arg("-A");
    }
    command
}

fn parse_os_release_kv(contents: &str) -> std::collections::HashMap<String, String> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, raw) = line.split_once('=')?;
            Some((
                key.to_string(),
                raw.trim_matches(|ch| ch == '"' || ch == '\'').to_string(),
            ))
        })
        .collect()
}

pub fn check_arch_linux() -> bool {
    if Path::new("/etc/arch-release").exists() {
        return true;
    }
    let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let release = parse_os_release_kv(&os_release);
    if let Some(id) = release.get("ID") {
        let id_lower = id.to_lowercase();
        if id_lower == "arch"
            || id_lower == "garuda"
            || id_lower == "manjaro"
            || id_lower == "endeavouros"
        {
            return true;
        }
    }
    if let Some(id_like) = release.get("ID_LIKE") {
        if id_like
            .split_whitespace()
            .any(|id| id.eq_ignore_ascii_case("arch"))
        {
            return true;
        }
    }
    false
}

pub fn is_solus_linux() -> bool {
    if Path::new("/etc/solus-release").exists() {
        return true;
    }
    let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let release = parse_os_release_kv(&os_release);
    if let Some(id) = release.get("ID") {
        if id.eq_ignore_ascii_case("solus") {
            return true;
        }
    }
    if let Some(id_like) = release.get("ID_LIKE") {
        if id_like
            .split_whitespace()
            .any(|id| id.eq_ignore_ascii_case("solus"))
        {
            return true;
        }
    }
    false
}

/// ¿Hay un gestor de paquetes soportado (pacman o eopkg) para operar?
fn has_supported_package_manager() -> bool {
    check_arch_linux() || is_solus_linux()
}

pub fn instalar_paquetes(reporter: &dyn Reporter, paquetes: &[&str]) -> bool {
    if !has_supported_package_manager() {
        reporter.err("El sistema no es Arch Linux ni Solus. No se pueden instalar paquetes.");
        return false;
    }

    let faltantes: Vec<&str> = paquetes
        .iter()
        .filter(|&&p| !is_package_installed(p))
        .copied()
        .collect();

    if faltantes.is_empty() {
        reporter.ok("Todos los paquetes ya están instalados. Saltando...");
        return true;
    }

    reporter.info(&format!(
        "📦  Se instalarán {} paquetes faltantes:",
        faltantes.len()
    ));
    for pkg in &faltantes {
        reporter.step(&format!("→ {}", pkg));
    }

    if !reporter.confirm("¿Deseas continuar con la instalación de estos paquetes?") {
        reporter.warn("Instalación de paquetes cancelada por el usuario.");
        return false;
    }

    let pkg_list = faltantes.join(" ");
    let cmd = if check_arch_linux() {
        format!(
            "{} pacman -S --needed --noconfirm {}",
            sudo_prefix(),
            pkg_list
        )
    } else {
        if !run_shell(&format!("{} eopkg update-repo -y", sudo_prefix())) {
            reporter.err(
                "No se pudo actualizar la lista de repositorios de Solus (eopkg update-repo).",
            );
            return false;
        }
        format!("{} eopkg install -y {}", sudo_prefix(), pkg_list)
    };
    reporter.progress("Instalando", 40);
    if !run_shell(&cmd) {
        reporter.err("Algunos paquetes no pudieron instalarse. Revisa la salida anterior.");
        return false;
    }
    true
}

/// ¿Conoce el gestor de paquetes este nombre en los repositorios activos?
///
/// Se usa para los extras que solo existen en repositorios de terceros (por
/// ejemplo Chaotic AUR): permite omitirlos con un aviso en vez de abortar toda
/// la instalacion con un "target not found".
///
/// Espera nombres de paquete REALES. `pacman -Si` no resuelve `provides`, asi
/// que un nombre virtual (p. ej. `dxvk`, que proveen `dxvk-mingw-git` y
/// `dxvk-async-git`) se daria por ausente. Pasa siempre el proveedor concreto:
/// ademas evita el prompt de seleccion de proveedor de pacman.
pub fn is_package_available(pkg: &str) -> bool {
    if check_arch_linux() {
        run_shell_piped(&format!("pacman -Si '{pkg}' >/dev/null 2>&1")).0
    } else if is_solus_linux() {
        run_shell_piped(&format!("eopkg info '{pkg}' >/dev/null 2>&1")).0
    } else {
        false
    }
}

/// Instala paquetes que pueden no estar en todos los repositorios soportados.
///
/// A diferencia de [`instalar_paquetes`], nunca aborta el flujo: descarta los
/// que el gestor no conoce, avisa de cada omision e instala el resto. Devuelve
/// `false` solo si fallo la instalacion de los que si estaban disponibles.
pub fn instalar_paquetes_opcionales(reporter: &dyn Reporter, paquetes: &[&str]) -> bool {
    if !has_supported_package_manager() {
        reporter.warn("Sin gestor de paquetes soportado: se omiten los extras opcionales.");
        return true;
    }

    let mut disponibles = Vec::new();
    for pkg in paquetes {
        if is_package_installed(pkg) || is_package_available(pkg) {
            disponibles.push(*pkg);
        } else {
            reporter.warn(&format!(
                "'{pkg}' no esta en tus repositorios; se omite (puedes instalarlo aparte)."
            ));
        }
    }

    if disponibles.is_empty() {
        return true;
    }
    instalar_paquetes(reporter, &disponibles)
}

pub fn desinstalar_paquetes(reporter: &dyn Reporter, paquetes: &[&str]) -> bool {
    if !has_supported_package_manager() {
        return false;
    }

    let a_eliminar: Vec<&str> = paquetes
        .iter()
        .filter(|&&p| is_package_installed(p))
        .copied()
        .collect();

    if a_eliminar.is_empty() {
        reporter.ok("Esos paquetes ya no están en el sistema. Saltando desinstalación...");
        return true;
    }

    reporter.info(&format!(
        "🗑️  Se van a desinstalar {} paquetes:",
        a_eliminar.len()
    ));
    for pkg in &a_eliminar {
        reporter.step(&format!("✗ {}", pkg));
    }

    let pkg_list = a_eliminar.join(" ");

    // `pacman -Rns` arrastra ademas las dependencias que dejan de ser
    // necesarias. El plan tiene que mostrar el conjunto REAL antes de pedir
    // confirmacion: antes se confirmaba una lista corta y se borraba otra mas
    // larga. `--print` no necesita privilegios y no toca el sistema.
    // Solo en Arch: `eopkg remove` no arrastra dependencias por su cuenta, asi
    // que en Solus la lista mostrada ya es el conjunto real.
    if check_arch_linux() {
        // `-Rns --print` no es valido (`--nosave` choca con `--print`); `-Rs`
        // calcula exactamente el mismo conjunto, porque `-n` solo afecta a si
        // se conservan los ficheros de configuracion.
        let (ok, salida) = run_shell_piped(&format!(
            "pacman -Rs --print --print-format '%n' {pkg_list} 2>&1"
        ));
        if ok {
            let arrastradas = salida
                .split_whitespace()
                .filter(|nombre| !a_eliminar.contains(nombre))
                .collect::<Vec<_>>();
            if !arrastradas.is_empty() {
                reporter.warn(&format!(
                    "Se retiraran tambien {} dependencias que dejan de ser necesarias:",
                    arrastradas.len()
                ));
                for pkg in &arrastradas {
                    reporter.step(&format!("✗ {} (dependencia)", pkg));
                }
            }
        } else {
            // Un fallo aqui suele significar que otro paquete instalado depende
            // de alguno de estos: se muestra tal cual, porque la desinstalacion
            // real fallara igual.
            reporter.warn("pacman no puede completar esta desinstalacion tal cual:");
            for linea in salida.lines().filter(|l| !l.trim().is_empty()).take(6) {
                reporter.step(linea.trim());
            }
        }
    }

    if !reporter.confirm("¿Deseas proceder con la desinstalación de estos paquetes?") {
        reporter.warn("Desinstalación cancelada por el usuario.");
        return false;
    }

    let cmd = if check_arch_linux() {
        format!("{} pacman -Rns --noconfirm {}", sudo_prefix(), pkg_list)
    } else {
        format!("{} eopkg remove -y {}", sudo_prefix(), pkg_list)
    };
    if !run_shell(&cmd) {
        reporter.err("Error al desinstalar paquetes.");
        return false;
    }
    true
}

pub fn configurar_fastfetch(reporter: &dyn Reporter) -> bool {
    reporter.info("Aplicando el tema universal de Fastfetch...");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let config_dir = format!("{}/.config/fastfetch", home);
    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        reporter.err(&format!(
            "No se pudo crear el directorio de fastfetch: {}",
            e
        ));
        return false;
    }

    // La confirmacion va ANTES de tocar nada: `resolve_fastfetch_logo` puede
    // copiar la imagen dentro de ~/.config/fastfetch, y no debe hacerlo si el
    // usuario acaba diciendo que no.
    let config_path = format!("{}/config.jsonc", config_dir);
    if Path::new(&config_path).exists() {
        reporter.info(&format!("Se sobrescribirá el archivo {}", config_path));
        if !reporter.confirm("¿Deseas sobrescribir config.jsonc de fastfetch?") {
            reporter.info("Se conserva tu config.jsonc de fastfetch.");
            return true;
        }
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        if let Err(e) = std::fs::copy(
            &config_path,
            format!("{}.backup_{}", config_path, timestamp),
        ) {
            reporter.err(&format!("Fallo al respaldar config.jsonc: {}", e));
            return false;
        }
    }

    // El logo solo se declara si la imagen existe de verdad: un `source`
    // inexistente hace que fastfetch imprima un error en cada shell interactiva.
    let logo_block = match resolve_fastfetch_logo(reporter, &config_dir) {
        Some(image) => format!(
            r#"  "logo": {{
    "type": "kitty-direct",
    "source": "{image}",
    "height": 20,
    "width": 30,
    "padding": {{ "top": 4, "left": 0, "right": 0 }}
  }},
"#
        ),
        None => String::new(),
    };

    let config = format!(
        r#"{{
  "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
{logo_block}  "display": {{
    "separator": " ► ",
    "bar": {{
      "char": {{ "elapsed": "", "total": "" }},
      "width": 15
    }},
    "percent": {{ "type": 2 }}
  }},
  "modules": [
    "break",
    {{ "type": "title", "format": " 👑 {{1}}@{{2}}", "keyColor": "35" }},
    {{ "type": "custom", "format": "───────────────────────────────────────────────────", "outputColor": "33" }},
    {{ "type": "custom", "format": " ✦ ✦ ✦  SYSTEM  ✦ ✦ ✦", "outputColor": "5" }},
    {{ "type": "os",       "key": " 🧩 OS  ", "keyColor": "yellow" }},
    {{ "type": "kernel",   "key": " ⚙️ KRNL", "keyColor": "yellow" }},
    {{ "type": "packages", "key": " 📦 PKGS", "keyColor": "yellow" }},
    {{ "type": "shell",    "key": " 🐚 SH  ", "keyColor": "yellow" }},
    {{ "type": "custom",   "format": " ✦ ✦ ✦  DESKTOP ✦ ✦ ✦", "outputColor": "5" }},
    {{ "type": "de",       "key": " 🪟 DE  ", "keyColor": "blue" }},
    {{ "type": "wm",       "key": " 🧭 WM  ", "keyColor": "blue" }},
    {{ "type": "lm",       "key": " 🔐 LM  ", "keyColor": "blue" }},
    {{ "type": "wmtheme",  "key": " 🎨 THM ", "keyColor": "blue" }},
    {{ "type": "icons",    "key": " 🧩 ICO ", "keyColor": "blue", "format": "{{1}}" }},
    {{ "type": "terminal", "key": " 🖥️ TERM", "keyColor": "blue" }},
    {{ "type": "custom",   "format": " ✦ ✦ ✦  Hardware  ✦ ✦ ✦", "outputColor": "5" }},
    {{ "type": "host",     "key": " 💻 HST ", "keyColor": "green" }},
    {{ "type": "cpu",      "key": " 🧠 CPU ", "keyColor": "green", "format": "{{1}}" }},
    {{ "type": "gpu",      "key": " 🎮 GPU ", "keyColor": "green" }},
    {{ "type": "disk",     "key": " 💾 DSK ", "keyColor": "green" }},
    {{ "type": "memory",   "key": " 🧬 RAM ", "keyColor": "green" }},
    {{ "type": "swap",     "key": " 🔄 SWP ", "keyColor": "green" }},
    {{ "type": "custom",   "format": " ✦ ✦ ✦  Media  ✦ ✦ ✦", "outputColor": "5" }},
    {{ "type": "sound",    "key": " 🔊 VOL ", "keyColor": "cyan" }},
    {{ "type": "player",   "key": " 🎵 PLY ", "keyColor": "cyan" }},
    {{ "type": "media",    "key": " 🎬 MED ", "keyColor": "cyan" }}
  ]
}}
"#,
        logo_block = logo_block
    );

    if let Err(e) = std::fs::write(&config_path, config) {
        reporter.err(&format!("Fallo al escribir config.jsonc: {}", e));
        return false;
    }
    reporter.ok("Configuración .jsonc de Fastfetch generada.");

    true
}

/// Resuelve la imagen del logo de fastfetch, o `None` para usar el logo de la
/// distribucion.
///
/// Busca `Anime Render.png` ya instalada en `~/.config/fastfetch/`; si no esta,
/// la copia desde el directorio de trabajo cuando el usuario la dejo ahi. Si no
/// aparece por ningun lado se omite el bloque `logo` del config, en vez de
/// apuntar a un archivo inexistente.
fn resolve_fastfetch_logo(reporter: &dyn Reporter, config_dir: &str) -> Option<String> {
    const LOGO_NAME: &str = "Anime Render.png";
    let destination = format!("{config_dir}/{LOGO_NAME}");
    if Path::new(&destination).exists() {
        return Some(destination);
    }
    let source = format!("./{LOGO_NAME}");
    if Path::new(&source).exists() {
        match std::fs::copy(&source, &destination) {
            Ok(_) => {
                reporter.ok("Imagen 'Anime Render.png' copiada a ~/.config/fastfetch/");
                return Some(destination);
            }
            Err(e) => reporter.warn(&format!("No se pudo copiar la imagen: {}", e)),
        }
    }
    reporter.info(&format!(
        "Sin '{LOGO_NAME}' en {config_dir}: fastfetch usara el logo de tu distribucion. \
         Copia ahi tu imagen y vuelve a ejecutar el preset para usarla."
    ));
    None
}

/// Prints the detected environment summary through the reporter.
pub fn print_detected_environment(
    reporter: &dyn Reporter,
    environment: &crate::environment::SystemEnvironment,
) {
    reporter.info("Entorno detectado");
    println!(
        "  {}Distribucion:{} {} ({})",
        DIM, RESET, environment.distro_name, environment.distro_id
    );
    println!(
        "  {}Arquitectura:{} {}",
        DIM, RESET, environment.architecture
    );
    println!("  {}Sesion:{} {}", DIM, RESET, environment.session);
    println!("  {}Escritorio:{} {}", DIM, RESET, environment.desktop);
    println!(
        "  {}Servicios:{} {}",
        DIM, RESET, environment.service_manager
    );
    println!(
        "  {}Paquetes:{} {}",
        DIM, RESET, environment.package_manager
    );
    if environment.compatibility.supported {
        reporter.ok("Entorno compatible con la primera version de Kito.");
    } else {
        reporter.warn("El entorno no esta soportado completamente.");
        for reason in &environment.compatibility.reasons {
            println!("    {}- {}{}", FG_YELLOW, reason, RESET);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_os_release_for_solus() {
        let content = "NAME=\"Solus\"\nID=\"solus\"\nPRETTY_NAME=\"Solus 4.5\"\n";
        let kv = parse_os_release_kv(content);
        assert_eq!(kv.get("ID").unwrap(), "solus");
        assert_eq!(kv.get("NAME").unwrap(), "Solus");
    }

    #[test]
    fn parses_id_like_arch_with_quotes() {
        let content = "NAME=\"Garuda Linux\"\nID=\"garuda\"\nID_LIKE=\"arch\"\n";
        let kv = parse_os_release_kv(content);
        assert_eq!(kv.get("ID").unwrap(), "garuda");
        assert_eq!(kv.get("ID_LIKE").unwrap(), "arch");
    }
}
