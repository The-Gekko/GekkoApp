//! Integracion con Matugen (Material You).
//!
//! GekkoApp no instala nada: si el sistema ya usa matugen (QuickShell/HyDE,
//! que regenera `~/.cache/matugen/colors-gtk.css` al cambiar el wallpaper),
//! GekkoApp detecta esa paleta y la aplica a su propia interfaz (Control
//! Center) y a las aplicaciones GTK3/GTK4 con las que convive. Si no hay
//! paleta disponible, se mantiene el tema por defecto.
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime};

pub const PALETTE_FILE: &str = "colors-gtk.css";

/// Intervalo de sondeo del archivo de paleta (matugen no notifica cambios).
const POLL_INTERVAL: Duration = Duration::from_millis(1000);

/// Ruta al CSS de colores que matugen genera para GTK (template `gtk` de HyDE).
pub fn palette_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/root"));
    let home_path = home.join(".cache/matugen").join(PALETTE_FILE);
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        let xdg_path = PathBuf::from(xdg).join("matugen").join(PALETTE_FILE);
        if xdg_path != home_path && xdg_path.exists() {
            return xdg_path;
        }
    }
    home_path
}

/// Paleta de colores detectada del sistema.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatugenPalette {
    pub available: bool,
    pub source: String,
    pub dark: bool,
    pub colors: BTreeMap<String, String>,
}

impl MatugenPalette {
    fn unavailable() -> Self {
        Self {
            available: false,
            source: String::new(),
            dark: true,
            colors: BTreeMap::new(),
        }
    }
}

fn parse_define_colors(content: &str) -> BTreeMap<String, String> {
    let mut colors = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("@define-color") else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let value = value.trim_end_matches(';').trim();
        if value.starts_with('#') {
            colors.insert(name.to_string(), value.to_string());
        }
    }
    colors
}

fn relative_luminance(hex: &str) -> Option<f64> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let channel = |i: usize| -> Option<f64> {
        let raw = u8::from_str_radix(&hex[i..i + 2], 16).ok()? as f64 / 255.0;
        Some(if raw <= 0.04045 {
            raw / 12.92
        } else {
            ((raw + 0.055) / 1.055).powf(2.4)
        })
    };
    let (r, g, b) = (channel(0)?, channel(2)?, channel(4)?);
    Some(0.2126 * r + 0.7152 * g + 0.0722 * b)
}

fn is_dark_palette(colors: &BTreeMap<String, String>) -> bool {
    let bg = colors
        .get("window_bg_color")
        .or_else(|| colors.get("theme_bg_color"))
        .or_else(|| colors.get("base_color"));
    match bg.and_then(|hex| relative_luminance(hex)) {
        Some(luminance) => luminance < 0.5,
        None => true,
    }
}

/// Detecta la paleta de matugen actual leyendo `~/.cache/matugen/colors-gtk.css`.
pub fn detect_palette() -> MatugenPalette {
    let path = palette_path();
    let Ok(content) = fs::read_to_string(&path) else {
        return MatugenPalette::unavailable();
    };
    let colors = parse_define_colors(&content);
    if colors.is_empty() {
        return MatugenPalette::unavailable();
    }
    MatugenPalette {
        available: true,
        source: path.display().to_string(),
        dark: is_dark_palette(&colors),
        colors,
    }
}

/// Vigila el archivo de paleta y llama a `callback` cada vez que cambia
/// (p. ej. cuando el wallpaper se regenera con matugen). La GUI lo usa para
/// re-tematizar en vivo, igual que el shell/dock de HyDE.
pub fn watch_palette<F>(mut callback: F)
where
    F: FnMut(MatugenPalette) + Send + 'static,
{
    thread::spawn(move || {
        let path = palette_path();
        let mut last = detect_palette();
        let mut last_modified = modified_time(&path);
        loop {
            thread::sleep(POLL_INTERVAL);
            let modified = modified_time(&path);
            if modified != last_modified {
                last_modified = modified;
                let palette = detect_palette();
                if palette.available || last.available {
                    callback(palette.clone());
                    last = palette;
                }
            }
        }
    });
}

fn modified_time(path: &std::path::Path) -> Option<SystemTime> {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_define_colors() {
        let css = "/* c */\n@define-color window_bg_color #1a1111;\n@define-color accent_color #ffb3b1;\n";
        let colors = parse_define_colors(css);
        assert_eq!(
            colors.get("window_bg_color").map(String::as_str),
            Some("#1a1111")
        );
        assert_eq!(
            colors.get("accent_color").map(String::as_str),
            Some("#ffb3b1")
        );
        assert_eq!(colors.len(), 2);
    }

    #[test]
    fn detects_dark_from_background() {
        let mut dark = BTreeMap::new();
        dark.insert("window_bg_color".to_string(), "#1a1111".to_string());
        assert!(is_dark_palette(&dark));

        let mut light = BTreeMap::new();
        light.insert("window_bg_color".to_string(), "#f0dedd".to_string());
        assert!(!is_dark_palette(&light));
    }

    #[test]
    fn unavailable_palette_reports_no_colors() {
        let palette = MatugenPalette::unavailable();
        assert!(!palette.available);
        assert!(palette.colors.is_empty());
        assert!(palette.source.is_empty());
    }
}
