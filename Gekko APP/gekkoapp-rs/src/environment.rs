use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemEnvironment {
    pub distro_id: String,
    pub distro_name: String,
    pub distro_like: Vec<String>,
    pub architecture: String,
    pub session: String,
    pub desktop: String,
    pub shell: String,
    pub display_manager: String,
    pub greeter: String,
    pub sddm_installed: bool,
    pub service_manager: String,
    pub package_manager: String,
    pub compatibility: Compatibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compatibility {
    pub supported: bool,
    pub reasons: Vec<String>,
}

impl SystemEnvironment {
    pub fn detect() -> Self {
        let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
        let variables = env::vars().collect::<HashMap<_, _>>();
        let mut result = Self::from_sources(&os_release, &variables, command_exists("systemctl"));
        let display_manager = fs::read_link("/etc/systemd/system/display-manager.service")
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            });
        let greetd_config = fs::read_to_string("/etc/greetd/config.toml").unwrap_or_default();
        result.detect_integrations(
            command_exists("dms"),
            command_exists("caelestia"),
            display_manager.as_deref(),
            &greetd_config,
            command_exists("sddm") || Path::new("/usr/bin/sddm").is_file(),
        );
        result
    }

    fn from_sources(
        os_release: &str,
        variables: &HashMap<String, String>,
        has_systemctl: bool,
    ) -> Self {
        let release = parse_os_release(os_release);
        let distro_id = release
            .get("ID")
            .cloned()
            .unwrap_or_else(|| "unknown".into());
        let distro_name = release
            .get("PRETTY_NAME")
            .or_else(|| release.get("NAME"))
            .cloned()
            .unwrap_or_else(|| distro_id.clone());
        let distro_like = release
            .get("ID_LIKE")
            .map(|value| {
                value
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let session = detect_session(variables);
        let desktop = detect_desktop(variables);
        let service_manager = if has_systemctl {
            "systemd-user".to_string()
        } else {
            "desconocido".to_string()
        };
        let package_manager = detect_package_manager(&distro_id, &distro_like);

        let mut result = Self {
            distro_id,
            distro_name,
            distro_like,
            architecture: env::consts::ARCH.to_string(),
            session,
            desktop,
            shell: "desconocido".into(),
            display_manager: "desconocido".into(),
            greeter: "desconocido".into(),
            sddm_installed: false,
            service_manager,
            package_manager,
            compatibility: Compatibility {
                supported: false,
                reasons: Vec::new(),
            },
        };
        result.refresh_compatibility();
        result
    }

    pub fn refresh_compatibility(&mut self) {
        let mut reasons = Vec::new();
        if self.distro_id != "arch" && !self.distro_like.iter().any(|id| id == "arch") {
            reasons.push("la primera version solo soporta Arch Linux y derivadas".into());
        }
        if self.architecture != "x86_64" {
            reasons.push("la primera version solo publica artefactos x86_64".into());
        }
        if self.session != "wayland" {
            reasons.push("se requiere una sesion Wayland".into());
        }
        match self.desktop.as_str() {
            "hyprland" | "niri" => {}
            _ => reasons.push("no existe un adaptador validado para este compositor".into()),
        }
        if self.service_manager != "systemd-user" {
            reasons.push("se requiere systemd para servicios de usuario".into());
        }
        self.compatibility = Compatibility {
            supported: reasons.is_empty(),
            reasons,
        };
    }

    fn detect_integrations(
        &mut self,
        has_dms: bool,
        has_caelestia: bool,
        display_manager_unit: Option<&str>,
        greetd_config: &str,
        has_sddm: bool,
    ) {
        self.sddm_installed = has_sddm;
        // Availability is not proof that the shell IPC is running.
        self.shell = match (has_dms, has_caelestia) {
            (true, true) => "dms, caelestia",
            (true, false) => "dms",
            (false, true) => "caelestia",
            _ => "desconocido",
        }
        .into();
        self.display_manager = match display_manager_unit {
            Some("greetd.service") => "greetd",
            Some("sddm.service") => "sddm",
            Some("gdm.service") => "gdm",
            Some("lightdm.service") => "lightdm",
            _ => "desconocido",
        }
        .into();
        self.greeter = if self.display_manager == "greetd" {
            let mut default_session = false;
            let is_dms = greetd_config.lines().any(|line| {
                let line = line.trim();
                if line.starts_with('[') {
                    default_session =
                        line.split('#').next().unwrap_or("").trim() == "[default_session]";
                    return false;
                }
                if !default_session {
                    return false;
                }
                let Some((key, value)) = line.split_once('=') else {
                    return false;
                };
                if key.trim() != "command" {
                    return false;
                }
                let value = value.trim();
                // Recognize only a simple quoted command. Unknown TOML forms
                // remain unknown rather than attributing a different greeter.
                let Some(quote) = value.chars().next().filter(|ch| *ch == '"' || *ch == '\'')
                else {
                    return false;
                };
                let Some(command) = value[1..].split(quote).next() else {
                    return false;
                };
                let program = command.split_whitespace().next().unwrap_or("");
                Path::new(program)
                    .file_name()
                    .is_some_and(|name| name == "dms-greeter")
            });
            if is_dms {
                "dms-greeter"
            } else {
                "desconocido"
            }
        } else if self.display_manager == "sddm" {
            "sddm"
        } else {
            "desconocido"
        }
        .into();
    }

    pub fn supports_kisddm(&self) -> bool {
        self.sddm_installed
    }

    pub fn target(&self) -> Option<&'static str> {
        match self.architecture.as_str() {
            "x86_64" => Some("x86_64-unknown-linux-gnu"),
            "aarch64" => Some("aarch64-unknown-linux-gnu"),
            _ => None,
        }
    }
}

fn parse_os_release(contents: &str) -> HashMap<String, String> {
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

fn detect_session(variables: &HashMap<String, String>) -> String {
    variables
        .get("XDG_SESSION_TYPE")
        .map(|value| value.to_ascii_lowercase())
        .or_else(|| variables.get("WAYLAND_DISPLAY").map(|_| "wayland".into()))
        .or_else(|| variables.get("DISPLAY").map(|_| "x11".into()))
        .unwrap_or_else(|| "desconocida".into())
}

fn detect_desktop(variables: &HashMap<String, String>) -> String {
    if variables.contains_key("HYPRLAND_INSTANCE_SIGNATURE") {
        return "hyprland".into();
    }
    if variables.contains_key("NIRI_SOCKET") {
        return "niri".into();
    }
    let desktop = variables
        .get("XDG_CURRENT_DESKTOP")
        .or_else(|| variables.get("XDG_SESSION_DESKTOP"))
        .or_else(|| variables.get("DESKTOP_SESSION"))
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    if desktop.contains("hyprland") {
        "hyprland".into()
    } else if desktop.contains("niri") {
        "niri".into()
    } else if desktop.contains("kde") || desktop.contains("plasma") {
        "kde".into()
    } else if desktop.contains("gnome") {
        "gnome".into()
    } else if desktop.is_empty() {
        "desconocido".into()
    } else {
        desktop
    }
}

fn detect_package_manager(distro_id: &str, distro_like: &[String]) -> String {
    if distro_id == "arch" || distro_like.iter().any(|id| id == "arch") {
        "pacman".into()
    } else if distro_id == "ubuntu"
        || distro_id == "debian"
        || distro_like.iter().any(|id| id == "debian")
    {
        "apt".into()
    } else if distro_id == "fedora" || distro_like.iter().any(|id| id == "fedora") {
        "dnf".into()
    } else {
        "desconocido".into()
    }
}

fn command_exists(command: &str) -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| Path::new(&directory).join(command).is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn niri_dms_greetd_are_supported_without_sddm() {
        let vars = HashMap::from([
            ("XDG_SESSION_TYPE".into(), "wayland".into()),
            ("NIRI_SOCKET".into(), "/run/user/1000/niri.sock".into()),
        ]);
        let mut environment = SystemEnvironment::from_sources("ID=arch", &vars, true);
        environment.detect_integrations(
            true,
            false,
            Some("greetd.service"),
            "[default_session]\ncommand = \"/usr/bin/dms-greeter --command niri\"\n",
            false,
        );
        assert_eq!(environment.desktop, "niri");
        assert_eq!(environment.shell, "dms");
        assert_eq!(environment.display_manager, "greetd");
        assert_eq!(environment.greeter, "dms-greeter");
        assert!(!environment.supports_kisddm());
        assert!(environment.compatibility.supported);
    }

    #[test]
    fn installed_shell_does_not_determine_display_manager() {
        let mut environment = SystemEnvironment::from_sources("ID=arch", &HashMap::new(), true);
        environment.detect_integrations(true, false, Some("sddm.service"), "", true);
        assert_eq!(environment.shell, "dms");
        assert!(environment.supports_kisddm());
        environment.detect_integrations(true, false, None, "command = \"dms-greeter\"", false);
        assert!(!environment.supports_kisddm());
        assert_eq!(environment.greeter, "desconocido");
    }

    #[test]
    fn kisddm_requires_installed_sddm_not_the_configured_login_manager() {
        let mut environment = SystemEnvironment::from_sources("ID=arch", &HashMap::new(), true);
        environment.detect_integrations(true, false, Some("greetd.service"), "", true);
        assert!(environment.supports_kisddm());
        assert_eq!(environment.display_manager, "greetd");
        environment.detect_integrations(false, false, Some("sddm.service"), "", false);
        assert!(!environment.supports_kisddm());
    }

    #[test]
    fn detects_supported_arch_hyprland_environment() {
        let vars = HashMap::from([
            ("XDG_SESSION_TYPE".into(), "wayland".into()),
            ("HYPRLAND_INSTANCE_SIGNATURE".into(), "instance".into()),
        ]);
        let environment =
            SystemEnvironment::from_sources("ID=arch\nPRETTY_NAME=\"Arch Linux\"\n", &vars, true);

        assert_eq!(environment.desktop, "hyprland");
        assert_eq!(environment.package_manager, "pacman");
        assert!(environment.compatibility.supported);
    }

    #[test]
    fn reports_each_unsupported_dimension() {
        let vars = HashMap::from([
            ("XDG_SESSION_TYPE".into(), "x11".into()),
            ("XDG_CURRENT_DESKTOP".into(), "GNOME".into()),
        ]);
        let environment =
            SystemEnvironment::from_sources("ID=ubuntu\nID_LIKE=debian\n", &vars, false);

        assert!(!environment.compatibility.supported);
        assert_eq!(environment.package_manager, "apt");
        assert_eq!(environment.compatibility.reasons.len(), 4);
    }
}
