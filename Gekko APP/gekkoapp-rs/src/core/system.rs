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
