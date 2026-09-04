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

# ---------------------------------------------------------------------------
# Configuracion y rutas XDG (espejan src/installer.rs)
# ---------------------------------------------------------------------------
PREFIX="${GEKKOAPP_PREFIX:-$HOME/.local}"
VERSION=""
LAUNCH=1
MODE="install"

usage() {
  # No se lee de "$0": bajo el one-liner documentado (`curl ... | bash`) "$0"
  # es "bash" y la ayuda saldria vacia o con el contenido equivocado.
  cat <<'AYUDA'
install-release.sh — Instala el Control Center de GekkoApp (GUI) desde el
release firmado de GitHub, sin compilar.

  --version <vX.Y.Z>   Instala una version concreta (por defecto: ultima).
  --prefix <dir>       Prefijo de instalacion (default: $HOME/.local).
  --no-launch          No abrir el Control Center al terminar.
  --uninstall          Desinstala la version instalada.
  --help               Muestra esta ayuda.

Seguridad: solo HTTPS y verificacion del SHA-256 del artefacto contra su
manifiesto antes de tocar el sistema.
AYUDA
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
# `installer.rs` (InstallPaths::detect) fija la raiz de versiones en
# $HOME/.local/lib/kitotsu y NO consulta XDG_LIB_HOME. El script tiene que usar
# exactamente la misma ruta o Rust y el script gestionarian arboles distintos.
VERSIONS_HOME="$HOME_DIR/.local/lib/kitotsu"
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
# ¿Este enlace apunta a alguna version instalada de GekkoApp?
owned_link() {
  local link="$1" target
  [ -L "$link" ] || return 1
  # `readlink -m` en vez de `-f`: `-f` devuelve vacio cuando el enlace esta roto
  # (la version a la que apunta ya no existe), y entonces un enlace nuestro se
  # tomaba por ajeno y se quedaba colgando.
  target="$(readlink -m "$link" 2>/dev/null || true)"
  [ -n "$target" ] && [ "${target#"$PRODUCT_HOME"/}" != "$target" ]
}

uninstall() {
  # Se compara contra $PRODUCT_HOME entero: adivinar la "version activa" por
  # orden lexicografico dejaba el enlace sin borrar y colgando tras el rm -rf.
  local link
  for link in "$BIN_HOME/$GUI_NAME" "$BIN_HOME/gekkoapp"; do
    if owned_link "$link"; then
      rm -f "$link"
      info "eliminado $link"
    elif [ -L "$link" ]; then
      say "  aviso: $link no apunta a GekkoApp; no se toca"
    fi
  done
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
case "$RELEASE_BASE" in
  https://*) ;;
  *) fail "solo se permiten descargas HTTPS: $RELEASE_BASE" ;;
esac

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

# Los campos del manifiesto se usan como nombre de archivo y de directorio:
# se exige que sean nombres simples, sin separadores de ruta.
for field in "$VERSION_FIELD" "$ARCHIVE_NAME"; do
  case "$field" in
    ""|*/*|.|..) fail "el manifiesto declara un valor no valido: '$field'" ;;
  esac
done

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
# Un enlace que apunte a CUALQUIER version bajo $PRODUCT_HOME es nuestro: antes
# se exigia que apuntase ya a la version nueva, asi que toda actualizacion
# terminaba en "no se sobreescribira una ruta ajena".
GUI_LINK="$BIN_HOME/$GUI_NAME"
DESIRED_TARGET="$(readlink -m "$FINAL_ROOT/bin/$GUI_NAME")"
if [ -L "$GUI_LINK" ]; then
  # `-m` resuelve tambien los enlaces rotos: con `-f`, un enlace a una version
  # ya borrada devolvia vacio y abortaba la instalacion como "ruta ajena".
  CURRENT_TARGET="$(readlink -m "$GUI_LINK" 2>/dev/null || true)"
  if [ "$CURRENT_TARGET" != "$DESIRED_TARGET" ] \
     && [ "${CURRENT_TARGET#"$PRODUCT_HOME"/}" = "$CURRENT_TARGET" ]; then
    fail "no se sobreescribira una ruta ajena: $GUI_LINK -> ${CURRENT_TARGET:-?}"
  fi
elif [ -e "$GUI_LINK" ]; then
  # Instalacion previa desde el codigo fuente (scripts/install.sh copia un
  # binario regular). Es nuestra: se adopta, igual que hace installer.rs.
  info "Reemplazando el $GUI_NAME instalado desde el codigo fuente."
fi
ln -sfn "$FINAL_ROOT/bin/$GUI_NAME" "$GUI_LINK"
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
