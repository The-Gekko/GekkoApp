#!/usr/bin/env bash

# ==============================================================================
# GekkoApp - Control Center (Tauri v2)
# ==============================================================================
# Punto de entrada principal: abre el Control Center de escritorio, desde el
# que se instala y actualiza todo el entorno (Kito, Bauh Fork, Gekko ADB Studio,
# el propio GekkoApp, Terminal Bonita, presets Hyprland/Niri, gaming y Chaotic
# AUR). Tambien notifica las nuevas actualizaciones (campana).
#
#   ./GekkoApp.sh            -> Control Center (GUI)
#   ./GekkoApp.sh --gui      -> (equivalente al modo por defecto)
#   ./GekkoApp.sh --cli      -> menu en terminal (SSH / power users)
#
# Si no hay binarios compilados, se compilan con cargo la primera vez.
# No hay fallback en bash: el comportamiento lo define siempre el Rust.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_APP_DIR="$SCRIPT_DIR/Gekko APP/gekkoapp-rs"
if [ ! -d "$RUST_APP_DIR" ]; then
    RUST_APP_DIR="$SCRIPT_DIR/gekkoapp-rs"
fi
CLI_BIN="$RUST_APP_DIR/target/release/gekkoapp"
GUI_BIN="$RUST_APP_DIR/target/release/gekkoapp-gui"

if [ "${1:-}" = "--cli" ]; then
    shift
    if [ -x "$CLI_BIN" ]; then
        exec "$CLI_BIN" "$@"
    fi
    if command -v cargo >/dev/null 2>&1 && [ -d "$RUST_APP_DIR" ]; then
        echo "Iniciando GekkoApp (Rust)..." >&2
        cd "$RUST_APP_DIR" && exec cargo run --release -- "$@"
    fi
    echo "No se pudo lanzar la CLI: falta el binario Rust o cargo." >&2
    exit 1
fi

# Modo por defecto: Control Center (GUI).
if [ -x "$GUI_BIN" ]; then
    exec "$GUI_BIN"
fi
if command -v cargo >/dev/null 2>&1 && [ -d "$RUST_APP_DIR" ]; then
    echo "Iniciando GekkoApp Control Center (Rust)..."
    cd "$RUST_APP_DIR" && exec cargo run --release --features gui --bin gekkoapp-gui
fi
echo "No se pudo lanzar el Control Center: falta la interfaz 'gui' compilada." >&2
exit 1
