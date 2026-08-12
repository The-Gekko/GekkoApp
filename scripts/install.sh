#!/usr/bin/env bash
#
# install.sh — Instala GekkoApp (CLI + Control Center GUI) desde el codigo fuente.
#
# Uso:
#   ./scripts/install.sh
#
# Variables de entorno:
#   GEKKOAPP_PREFIX   Prefijo de instalacion (default: $HOME/.local)
#   GEKKOAPP_SKIP_BUILD=1   No reconstruir si los binarios ya existen
#
# Instala en el prefijo:
#   bin/gekkoapp          CLI (Rust)
#   bin/gekkoapp-gui      Control Center (Rust + Tauri v2)
#   share/applications/gekkoapp-control-center.desktop
#   share/icons/hicolor/512x512/apps/org.thegekko.gekkoapp.png
#
# Idempotente: re-ejecutable sin efectos secundarios.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CRATE_DIR="$ROOT_DIR/Gekko APP/gekkoapp-rs"
ICON_SRC="$CRATE_DIR/icons/icon.png"
DESKTOP_TEMPLATE="$ROOT_DIR/packaging/gekkoapp-control-center.desktop"

PREFIX="${GEKKOAPP_PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
DATA_DIR="${XDG_DATA_HOME:-$PREFIX/share}"
APPS_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor/512x512/apps"

APP_ID="org.thegekko.gekkoapp"

c_print() { printf '%s\n' "$*"; }

fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[ -d "$CRATE_DIR" ] || fail "no se encuentra el crate en $CRATE_DIR"
[ -f "$DESKTOP_TEMPLATE" ] || fail "no se encuentra la plantilla .desktop en $DESKTOP_TEMPLATE"
command -v cargo >/dev/null 2>&1 || fail "se requiere 'cargo' (instala rustup: https://rustup.rs)"

echo "==> Prefijo de instalacion: $PREFIX"

if [ "${GEKKOAPP_SKIP_BUILD:-0}" != "1" ]; then
  echo "==> Compilando CLI (release, --locked)..."
  cargo build --locked --release --manifest-path "$CRATE_DIR/Cargo.toml"

  echo "==> Compilando Control Center (release, feature 'gui')..."
  cargo build --locked --release --features gui --bin gekkoapp-gui --manifest-path "$CRATE_DIR/Cargo.toml"
else
  echo "==> Omitiendo build (GEKKOAPP_SKIP_BUILD=1)"
fi

CLI_BIN="$CRATE_DIR/target/release/gekkoapp"
GUI_BIN="$CRATE_DIR/target/release/gekkoapp-gui"
[ -x "$CLI_BIN" ] || fail "no se encontro el binario CLI: $CLI_BIN"
[ -x "$GUI_BIN" ] || fail "no se encontro el binario GUI: $GUI_BIN"
[ -f "$ICON_SRC" ] || fail "no se encontro el icono: $ICON_SRC"

mkdir -p "$BIN_DIR" "$APPS_DIR" "$ICON_DIR"

echo "==> Instalando binarios..."
install -m 0755 "$CLI_BIN" "$BIN_DIR/gekkoapp"
install -m 0755 "$GUI_BIN" "$BIN_DIR/gekkoapp-gui"

echo "==> Instalando icono ($APP_ID)..."
install -m 0644 "$ICON_SRC" "$ICON_DIR/$APP_ID.png"

echo "==> Generando e instalando entrada de aplicacion..."
sed -e "s|__GEKKOAPP_GUI_BIN__|$BIN_DIR/gekkoapp-gui|" "$DESKTOP_TEMPLATE" > "$APPS_DIR/gekkoapp-control-center.desktop"
chmod 0644 "$APPS_DIR/gekkoapp-control-center.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
fi

echo
echo "==> Instalacion completada."
echo "  CLI:               $BIN_DIR/gekkoapp"
echo "  Control Center:    $BIN_DIR/gekkoapp-gui"
echo "  Entrada de menu:   $APPS_DIR/gekkoapp-control-center.desktop"

if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
  echo
  echo "AVISO: $BIN_DIR no esta en tu PATH. Añádelo, por ejemplo:"
  echo "  echo 'export PATH=\"\$PATH:$BIN_DIR\"' >> ~/.bashrc"
fi
