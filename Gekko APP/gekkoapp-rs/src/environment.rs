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
        Self::from_sources(&os_release, &variables, command_exists("systemctl"))
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
        if self.desktop != "hyprland" {
            reasons.push("el primer adaptador validado es Hyprland".into());
        }
        if self.service_manager != "systemd-user" {
            reasons.push("se requiere systemd para servicios de usuario".into());
        }
        self.compatibility = Compatibility {
            supported: reasons.is_empty(),
            reasons,
        };
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
