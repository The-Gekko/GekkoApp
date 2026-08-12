#!/usr/bin/env bash
#
# install-release.sh — Instala el Control Center de GekkoApp (GUI) desde el
# release firmado de GitHub, sin compilar. Es la forma recomendada para
# usuarios finales.
#
# Uso:
#   curl -fsSL https://raw.githubusercontent.com/The-Gekko/GekkoApp/main/scripts/install-release.sh | bash
#
#   --version <vX.Y.Z>   Instala una version concreta (por defecto: ultima).
#   --prefix <dir>       Prefijo de instalacion (default: $HOME/.local).
#   --no-launch          No abrir el Control Center al terminar.
#   --uninstall          Desinstala la version instalada.
#   --help               Muestra esta ayuda.
#
# Seguridad: solo HTTPS, verifica el SHA-256 del artefacto contra su manifiesto
# (contrato kitotsu.release-artifact) antes de tocar el sistema. No instala
# ningun componente (Kito, Bauh, Gekko ADB, terminal, gaming, Chaotic AUR):
# eso se hace desde dentro del Control Center. Idempotente.

set -euo pipefail

REPO="The-Gekko/GekkoApp"
TARGET="x86_64-unknown-linux-gnu"
APP_ID="org.thegekko.gekkoapp"
PRODUCT="gekkoapp"
RAW_BASE="https://raw.githubusercontent.com/$REPO/main/scripts/install-release.sh"

# ---------------------------------------------------------------------------
# Configuracion y rutas XDG (espejan src/installer.rs)
# ---------------------------------------------------------------------------
PREFIX="${GEKKOAPP_PREFIX:-$HOME/.local}"
VERSION=""
LAUNCH=1
MODE="install"

usage() {
  sed -n '1,20p' "$0" | sed 's/^# \{0,1\}//' | sed '/^$/d'
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="${2:-}"; shift 2 ;;
    --prefix) PREFIX="${2:-}"; shift 2 ;;
    --no-launch) LAUNCH=0; shift ;;
    --uninstall) MODE="uninstall"; shift ;;
    --help) usage; exit 0 ;;
    *) echo "opcion desconocida: $1" >&2; usage; exit 2 ;;
  esac
done

HOME_DIR="${HOME:?se requiere \$HOME}"
DATA_HOME="${XDG_DATA_HOME:-$HOME_DIR/.local/share}"
BIN_HOME="${XDG_BIN_HOME:-$PREFIX/bin}"
VERSIONS_HOME="${XDG_LIB_HOME:-$HOME_DIR/.local/lib/kitotsu}"
PRODUCT_HOME="$VERSIONS_HOME/$PRODUCT"
APPS_DIR="$DATA_HOME/applications"
ICON_DIR="$DATA_HOME/icons/hicolor/512x512/apps"
SYMBOLIC_DIR="$DATA_HOME/icons/hicolor/symbolic/apps"
DESKTOP_FILE="$APPS_DIR/$APP_ID.desktop"
GUI_NAME="gekkoapp-gui"

require() {
  command -v "$1" >/dev/null 2>&1 || { echo "error: se requiere '$1'" >&2; exit 1; }
}

say()  { printf '%s\n' "$*"; }
info() { printf '==> %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

resolve_latest_tag() {
  if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    gh release view --repo "$REPO" --json tagName --jq .tagName 2>/dev/null && return
  fi
  curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["tag_name"])'
}

# ---------------------------------------------------------------------------
# Desinstalar
# ---------------------------------------------------------------------------
uninstall() {
  local active_root=""
  if [ -d "$PRODUCT_HOME" ]; then
    active_root="$(cd "$PRODUCT_HOME" && find . -mindepth 1 -maxdepth 1 -type d | sort | tail -n1)"
    [ -n "$active_root" ] && active_root="$PRODUCT_HOME/${active_root#./}"
  fi

  if [ -L "$BIN_HOME/$GUI_NAME" ]; then
    local target
    target="$(readlink -f "$BIN_HOME/$GUI_NAME")"
    if [ -n "$active_root" ] && [ "${target#"$active_root"/}" != "$target" ]; then
      rm -f "$BIN_HOME/$GUI_NAME"
      info "eliminado $BIN_HOME/$GUI_NAME"
    else
      say "  aviso: $BIN_HOME/$GUI_NAME no apunta a GekkoApp; no se toca"
    fi
  fi
  [ -f "$DESKTOP_FILE" ] && rm -f "$DESKTOP_FILE" && info "eliminado $DESKTOP_FILE"
  [ -f "$ICON_DIR/$APP_ID.png" ] && rm -f "$ICON_DIR/$APP_ID.png" && info "eliminado $ICON_DIR/$APP_ID.png"
  [ -f "$SYMBOLIC_DIR/$APP_ID-symbolic.svg" ] && rm -f "$SYMBOLIC_DIR/$APP_ID-symbolic.svg" && info "eliminado $SYMBOLIC_DIR/$APP_ID-symbolic.svg"
  if [ -d "$PRODUCT_HOME" ]; then
    rm -rf "$PRODUCT_HOME"
    info "eliminado $PRODUCT_HOME"
  fi
  refresh_desktop_db
  say "GekkoApp Control Center desinstalado."
  exit 0
}

refresh_desktop_db() {
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
  command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -f "$DATA_HOME/icons/hicolor" >/dev/null 2>&1 || true
}

# ---------------------------------------------------------------------------
# Instalar
# ---------------------------------------------------------------------------
[ "$MODE" = "uninstall" ] && uninstall

require curl
require tar
require python3

info "Resolviendo el ultimo release de $REPO..."
TAG="${VERSION:-$(resolve_latest_tag)}"
TAG="${TAG#v}"
if [ -z "$TAG" ]; then
  fail "no se pudo resolver el release. Publica un release primero (gh release create) o usa --version."
fi

# Base por release; GEKKOAPP_RELEASE_BASE permite espejos/probar en local.
RELEASE_BASE="${GEKKOAPP_RELEASE_BASE:-https://github.com/$REPO/releases/download/v$TAG}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
MANIFEST_URL="$RELEASE_BASE/$PRODUCT-$TARGET.manifest.json"

info "Descargando manifiesto firmado..."
curl -fsSL "$MANIFEST_URL" -o "$TMP/manifest.json" \
  || fail "no se pudo descargar el manifiesto $MANIFEST_URL"

VERSION_FIELD="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["product"]["version"])' "$TMP/manifest.json")"
ARCHIVE_NAME="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["artifact"]["file_name"])' "$TMP/manifest.json")"
ARCHIVE_SIZE="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["artifact"]["size_bytes"])' "$TMP/manifest.json")"
ARCHIVE_SHA="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["artifact"]["sha256"])' "$TMP/manifest.json")"

info "Version: $VERSION_FIELD"
info "Descargando artefacto ($ARCHIVE_NAME)..."
curl -fsSL "$RELEASE_BASE/$ARCHIVE_NAME" -o "$TMP/$ARCHIVE_NAME" \
  || fail "no se pudo descargar el artefacto"

ACTUAL_SIZE="$(stat -c %s "$TMP/$ARCHIVE_NAME")"
ACTUAL_SHA="$(sha256sum "$TMP/$ARCHIVE_NAME" | awk '{print $1}')"
[ "$ACTUAL_SIZE" = "$ARCHIVE_SIZE" ] || fail "el tamano del artefacto no coincide con el manifiesto"
[ "$ACTUAL_SHA" = "$ARCHIVE_SHA" ] || fail "la verificacion SHA-256 del artefacto fallo; se aborta por seguridad"
info "Verificacion SHA-256 correcta."

FINAL_ROOT="$PRODUCT_HOME/$VERSION_FIELD"
if [ -d "$FINAL_ROOT" ]; then
  info "La version $VERSION_FIELD ya esta instalada; actualizando la activacion."
else
  info "Extrayendo a $FINAL_ROOT ..."
  mkdir -p "$PRODUCT_HOME"
  tar --zstd -xf "$TMP/$ARCHIVE_NAME" -C "$TMP"
  EXTRACTED="$(find "$TMP" -mindepth 1 -maxdepth 1 -type d | head -n1)"
  mv "$EXTRACTED" "$FINAL_ROOT"
fi

[ -x "$FINAL_ROOT/bin/$GUI_NAME" ] || fail "el release no contiene bin/$GUI_NAME"

mkdir -p "$BIN_HOME" "$APPS_DIR" "$ICON_DIR" "$SYMBOLIC_DIR"

# Entrypoint: symlink propio en ~/.local/bin (no se pisa una ruta ajena).
GUI_LINK="$BIN_HOME/$GUI_NAME"
if [ -e "$GUI_LINK" ] || [ -L "$GUI_LINK" ]; then
  if [ "$(readlink -f "$GUI_LINK")" != "$(readlink -f "$FINAL_ROOT/bin/$GUI_NAME")" ]; then
    fail "no se sobreescribira una ruta ajena: $GUI_LINK"
  fi
fi
ln -sf "$FINAL_ROOT/bin/$GUI_NAME" "$GUI_LINK"
info "Instalado: $GUI_LINK"

# Entrada de menu materializada (token @EXECUTABLE@ -> ruta real).
sed -e "s|@EXECUTABLE@|\"$GUI_LINK\"|" \
    -e "s|@APPLICATION_ID@|$APP_ID|g" \
    "$FINAL_ROOT/gekkoapp-control-center.desktop" > "$DESKTOP_FILE"
chmod 0644 "$DESKTOP_FILE"
info "Instalado: $DESKTOP_FILE"

# Iconos hicolor (PNG 512 + simbolico).
install -m 0644 "$FINAL_ROOT/$APP_ID.png" "$ICON_DIR/$APP_ID.png"
install -m 0644 "$FINAL_ROOT/$APP_ID-symbolic.svg" "$SYMBOLIC_DIR/$APP_ID-symbolic.svg"
info "Instalados los iconos."

refresh_desktop_db

say ""
say "  GekkoApp Control Center $VERSION_FIELD instalado."
say "  GUI:    $GUI_LINK"
say "  Menu:   $DESKTOP_FILE"
say "  Desde el Control Center podras instalar y actualizar todo lo demas."
say ""

if [ "$LAUNCH" = "1" ]; then
  info "Abriendo el Control Center..."
  nohup "$GUI_LINK" </dev/null >/dev/null 2>&1 &
  disown
fi
