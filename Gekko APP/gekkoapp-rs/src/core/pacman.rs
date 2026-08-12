use crate::core::reporter::Reporter;
use crate::core::system::{check_arch_linux, run_shell, run_shell_piped};

fn check_chaotic_aur_configured(pacman_conf: &str) -> bool {
    let mut in_chaotic = false;
    let mut has_siglevel = false;
    let mut has_include = false;

    for line in pacman_conf.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if trimmed == "[chaotic-aur]" {
                in_chaotic = true;
                has_siglevel = false;
                has_include = false;
            } else if in_chaotic {
                if has_siglevel && has_include {
                    return true;
                }
                in_chaotic = false;
                has_siglevel = false;
                has_include = false;
            }
        } else if in_chaotic {
            if let Some((key, val)) = trimmed.split_once('=') {
                let key = key.trim();
                let val = val.trim();
                if key == "SigLevel" && val == "Required DatabaseOptional" {
                    has_siglevel = true;
                } else if key == "Include" && val == "/etc/pacman.d/chaotic-mirrorlist" {
                    has_include = true;
                }
            }
        }
    }

    in_chaotic && has_siglevel && has_include
}

enum ReplaceResult {
    ConcurrentModification,
    NotReplaced,
    Replaced,
}

struct LockGuard<'a> {
    path: &'a str,
}

impl<'a> Drop for LockGuard<'a> {
    fn drop(&mut self) {
        let st = std::process::Command::new("sudo")
            .args(["rmdir", self.path])
            .status();
        if let Ok(status) = st {
            if !status.success() {
                eprintln!("Atención: No se pudo liberar el candado en {}", self.path);
            }
        } else {
            eprintln!("Atención: Error ejecutando rmdir en {}", self.path);
        }
    }
}

struct BackupGuard {
    path: String,
    consumed: bool,
    remover: fn(&str) -> Result<(), ()>,
}

fn default_remover(path: &str) -> Result<(), ()> {
    match std::process::Command::new("sudo")
        .args(["rm", "-f", path])
        .status()
    {
        Ok(status) if status.success() => Ok(()),
        _ => Err(()),
    }
}

impl BackupGuard {
    fn new(path: String) -> Self {
        Self {
            path,
            consumed: false,
            remover: default_remover,
        }
    }

    #[cfg(test)]
    fn new_with_remover(path: String, remover: fn(&str) -> Result<(), ()>) -> Self {
        Self {
            path,
            consumed: false,
            remover,
        }
    }

    fn keep(&mut self, reason: &str) {
        self.consumed = true;
        eprintln!(
            "Conservando backup.\nRazón: {}\nRuta absoluta: {}\nSe requiere recuperación manual.",
            reason, self.path
        );
    }

    fn consume(&mut self) {
        self.consumed = true;
    }

    fn cleanup(&mut self) -> Result<(), ()> {
        if !self.consumed {
            match (self.remover)(&self.path) {
                Ok(_) => {
                    self.consumed = true;
                    Ok(())
                }
                Err(_) => Err(()),
            }
        } else {
            Ok(())
        }
    }
}

impl Drop for BackupGuard {
    fn drop(&mut self) {
        if !self.consumed && self.cleanup().is_err() {
            eprintln!(
                "Atención: Falló la eliminación automática del backup en {}",
                self.path
            );
        }
    }
}

fn replace_pacman_conf_securely(current_conf: &str, new_conf: &str) -> ReplaceResult {
    let mktemp_cmd = std::process::Command::new("sudo")
        .arg("mktemp")
        .arg("/etc/pacman.conf.tmp.XXXXXX")
        .output();

    let tmp_file = match mktemp_cmd {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => return ReplaceResult::NotReplaced,
    };

    let cleanup = |file: &str| {
        let _ = std::process::Command::new("sudo")
            .arg("rm")
            .arg("-f")
            .arg(file)
            .status();
    };

    let mut tee = match std::process::Command::new("sudo")
        .arg("tee")
        .arg(&tmp_file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    };

    if let Some(mut stdin) = tee.stdin.take() {
        if std::io::Write::write_all(&mut stdin, new_conf.as_bytes()).is_err() {
            let _ = tee.wait();
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    }

    match tee.wait() {
        Ok(status) if status.success() => {}
        _ => {
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    }

    match std::process::Command::new("sudo")
        .args(["chmod", "644", &tmp_file])
        .status()
    {
        Ok(status) if status.success() => {}
        _ => {
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    }
    match std::process::Command::new("sudo")
        .args(["chown", "root:root", &tmp_file])
        .status()
    {
        Ok(status) if status.success() => {}
        _ => {
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    }

    if !check_chaotic_aur_configured(new_conf) {
        cleanup(&tmp_file);
        return ReplaceResult::NotReplaced;
    }

    let active_conf = match std::fs::read_to_string("/etc/pacman.conf") {
        Ok(c) => c,
        Err(_) => {
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    };
    if active_conf != current_conf {
        cleanup(&tmp_file);
        return ReplaceResult::ConcurrentModification;
    }

    let mv_status = std::process::Command::new("sudo")
        .args(["mv", &tmp_file, "/etc/pacman.conf"])
        .status();

    if let Ok(st) = mv_status {
        if st.success() {
            ReplaceResult::Replaced
        } else {
            cleanup(&tmp_file);
            ReplaceResult::NotReplaced
        }
    } else {
        cleanup(&tmp_file);
        ReplaceResult::NotReplaced
    }
}

pub fn install_chaotic_aur(reporter: &dyn Reporter) -> bool {
    reporter.header("AGREGANDO REPOSITORIOS CHAOTIC AUR");

    if !check_arch_linux() {
        reporter.err("El sistema no es Arch Linux. No se pueden configurar los repositorios.");
        return false;
    }

    if !reporter.confirm("¿Deseas instalar y configurar Chaotic AUR?") {
        return false;
    }

    let lock_path = "/run/lock/gekkoapp_pacman_conf_lock";
    if !run_shell(&format!("sudo mkdir {}", lock_path)) {
        reporter.err("No se pudo obtener el bloqueo de transacción. ¿Otra instancia en ejecución?");
        return false;
    }
    let _lock = LockGuard { path: lock_path };

    let keyring_installed = run_shell("pacman -Qq chaotic-keyring >/dev/null 2>&1");
    let mirrorlist_installed = run_shell("pacman -Qq chaotic-mirrorlist >/dev/null 2>&1");

    let pacman_conf = match std::fs::read_to_string("/etc/pacman.conf") {
        Ok(conf) => conf,
        Err(_) => {
            reporter.err("Error crítico: No se pudo leer /etc/pacman.conf");
            return false;
        }
    };

    let repo_configured = check_chaotic_aur_configured(&pacman_conf);

    if keyring_installed && mirrorlist_installed && repo_configured {
        reporter.ok("Chaotic AUR ya está completamente configurado.");
        return true;
    }

    if !keyring_installed || !mirrorlist_installed {
        reporter.info("Obteniendo y verificando llaves públicas...");
        let key_success = run_shell("sudo pacman-key --recv-key 3056513887B78AEB --keyserver hkps://keys.openpgp.org || sudo pacman-key --recv-key 3056513887B78AEB --keyserver keyserver.ubuntu.com");
        if !key_success {
            reporter.err("Error crítico: No se pudo obtener la llave GPG de Chaotic AUR.");
            return false;
        }

        if !run_shell("sudo pacman-key --lsign-key 3056513887B78AEB") {
            reporter.err("Error al firmar la llave GPG.");
            return false;
        }

        reporter.info("Instalando keyring y mirrorlist de Chaotic AUR...");
        if !run_shell("sudo pacman -U --noconfirm 'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst' 'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst'") {
            reporter.err("Error instalando paquetes de Chaotic AUR.");
            return false;
        }
    }

    let current_conf = match std::fs::read_to_string("/etc/pacman.conf") {
        Ok(conf) => conf,
        Err(_) => {
            reporter.err("Error crítico: No se pudo releer /etc/pacman.conf");
            return false;
        }
    };

    if !check_chaotic_aur_configured(&current_conf) {
        reporter.info("Editando /etc/pacman.conf de manera segura...");
        let (ok, out) = run_shell_piped("sudo mktemp /etc/pacman.conf.backup.XXXXXX");
        let backup_file = if ok {
            out.trim().to_string()
        } else {
            reporter.err("No se pudo generar nombre seguro para el backup.");
            return false;
        };
        let mut backup_guard = BackupGuard::new(backup_file.clone());

        let mut tee = match std::process::Command::new("sudo")
            .arg("tee")
            .arg(&backup_file)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => return false,
        };
        if let Some(mut stdin) = tee.stdin.take() {
            if std::io::Write::write_all(&mut stdin, current_conf.as_bytes()).is_err() {
                let _ = tee.wait();
                return false;
            }
        }
        if !tee.wait().map(|s| s.success()).unwrap_or(false) {
            reporter.err("No se pudo respaldar pacman.conf.");
            return false;
        }

        let mut new_conf = String::new();
        let mut in_chaotic = false;
        for line in current_conf.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("[chaotic-aur]") {
                in_chaotic = true;
                continue;
            }
            if in_chaotic && trimmed.starts_with('[') {
                in_chaotic = false;
            }
            if !in_chaotic {
                new_conf.push_str(line);
                new_conf.push('\n');
            }
        }
        new_conf.push_str("\n[chaotic-aur]\nSigLevel = Required DatabaseOptional\nInclude = /etc/pacman.d/chaotic-mirrorlist\n");

        match replace_pacman_conf_securely(&current_conf, &new_conf) {
            ReplaceResult::ConcurrentModification => {
                reporter.err("Archivo pacman.conf modificado por otro proceso. Abortando.");
                return false;
            }
            ReplaceResult::NotReplaced => {
                reporter.err("Fallo al preparar o reemplazar pacman.conf.");
                return false;
            }
            ReplaceResult::Replaced => {
                reporter.info("Sincronizando repositorios y actualizando el sistema...");
                if !run_shell("sudo pacman -Syu") {
                    reporter.err("Error sincronizando repositorios. Restaurando backup...");
                    match std::fs::read_to_string("/etc/pacman.conf") {
                        Ok(active) => {
                            if active == new_conf {
                                let backup_ok = match std::process::Command::new("sudo")
                                    .args(["cat", "--", &backup_guard.path])
                                    .output()
                                {
                                    Ok(out) if out.status.success() => {
                                        out.stdout == current_conf.as_bytes()
                                    }
                                    _ => false,
                                };
                                if !backup_ok {
                                    reporter.err("ERROR CRÍTICO: El backup no coincide con la configuración original o no se pudo leer. Se aborta la restauración.");
                                    backup_guard.keep("El backup no coincide con la configuración original o no se pudo leer.");
                                } else {
                                    let mut perms_ok = true;
                                    if !run_shell(&format!("sudo chmod 644 {}", backup_guard.path))
                                    {
                                        perms_ok = false;
                                    }
                                    if !run_shell(&format!(
                                        "sudo chown root:root {}",
                                        backup_guard.path
                                    )) {
                                        perms_ok = false;
                                    }
                                    if perms_ok {
                                        let restore_cmd = format!(
                                            "sudo mv {} /etc/pacman.conf",
                                            backup_guard.path
                                        );
                                        if !run_shell(&restore_cmd) {
                                            reporter.err(
                                                "ERROR CRÍTICO: No se pudo restaurar pacman.conf.",
                                            );
                                            backup_guard
                                                .keep("Fallo al ejecutar mv de restauración.");
                                        } else {
                                            backup_guard.consume();
                                        }
                                    } else {
                                        reporter.err("ERROR CRÍTICO: No se pudieron establecer permisos en el backup.");
                                        backup_guard.keep("Fallo al establecer permisos 644/root:root en el backup.");
                                    }
                                }
                            } else {
                                reporter.warn("pacman.conf modificado tras nuestra escritura. Backup descartado.");
                            }
                        }
                        Err(e) => {
                            reporter.err(&format!(
                                "Error crítico leyendo pacman.conf en rollback: {}",
                                e
                            ));
                            backup_guard.keep("Error de lectura durante el rollback.");
                        }
                    }
                    return false;
                }
                if backup_guard.cleanup().is_err() {
                    eprintln!(
                        "Atención: Falló la eliminación explícita del backup en: {}",
                        backup_guard.path
                    );
                }
            }
        }
    } else {
        reporter.info("Sincronizando repositorios y actualizando el sistema...");
        if !run_shell("sudo pacman -Syu") {
            reporter.err("Error sincronizando repositorios (pacman -Syu falló).");
            return false;
        }
    }

    reporter.ok("¡Repositorios Chaotic AUR configurados con éxito!");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_chaotic_aur_configured() {
        let valid = "
[options]
Architecture = auto

[chaotic-aur]
SigLevel = Required DatabaseOptional
Include = /etc/pacman.d/chaotic-mirrorlist
        ";
        assert!(check_chaotic_aur_configured(valid));

        let incomplete = "
[chaotic-aur]
SigLevel = Required DatabaseOptional
        ";
        assert!(!check_chaotic_aur_configured(incomplete));

        let wrong_keys = "
[chaotic-aur]
NotSigLevel = Required DatabaseOptional
BadInclude = /etc/pacman.d/chaotic-mirrorlist
        ";
        assert!(!check_chaotic_aur_configured(wrong_keys));

        let multiple_blocks_no_merge = "
[chaotic-aur]
SigLevel = Required DatabaseOptional

[chaotic-aur]
Include = /etc/pacman.d/chaotic-mirrorlist
        ";
        assert!(!check_chaotic_aur_configured(multiple_blocks_no_merge));

        let commented = "
#[chaotic-aur]
#SigLevel = Required DatabaseOptional
#Include = /etc/pacman.d/chaotic-mirrorlist
        ";
        assert!(!check_chaotic_aur_configured(commented));

        let mixed_repos = "
[core]
Include = /etc/pacman.d/mirrorlist

[chaotic-aur]
SigLevel = Required DatabaseOptional
Include = /etc/pacman.d/chaotic-mirrorlist

[extra]
Include = /etc/pacman.d/mirrorlist
        ";
        assert!(check_chaotic_aur_configured(mixed_repos));
    }

    #[test]
    fn test_backup_guard_transitions() {
        let mut guard_keep =
            BackupGuard::new_with_remover("/tmp/test_keep".to_string(), |_| Ok(()));
        assert!(!guard_keep.consumed);
        guard_keep.keep("Test reason");
        assert!(guard_keep.consumed);

        let mut guard_consume =
            BackupGuard::new_with_remover("/tmp/test_consume".to_string(), |_| unreachable!());
        guard_consume.consume();
        assert!(guard_consume.consumed);
        assert_eq!(guard_consume.cleanup(), Ok(()));

        let mut guard_clean =
            BackupGuard::new_with_remover("/tmp/test_clean".to_string(), |_| Ok(()));
        assert!(!guard_clean.consumed);
        assert_eq!(guard_clean.cleanup(), Ok(()));
        assert!(guard_clean.consumed);

        let mut guard_fail =
            BackupGuard::new_with_remover("/tmp/test_fail".to_string(), |_| Err(()));
        assert!(!guard_fail.consumed);
        assert_eq!(guard_fail.cleanup(), Err(()));
        assert!(!guard_fail.consumed);
    }
}
