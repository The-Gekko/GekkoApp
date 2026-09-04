#!/usr/bin/env bash
#
# install-release.sh — Instala el Control Center de GekkoApp (GUI + CLI) desde
# el release verificado (manifiesto + SHA-256) de GitHub, sin compilar. Es la
# forma recomendada para usuarios finales.
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
# (contrato kitotsu.release-artifact) antes de tocar el sistema, y comprueba
# que la glibc del host alcanza la minima que declara el manifiesto. No instala
# ningun componente (Kito, Bauh, Gekko ADB, terminal, gaming, Chaotic AUR):
# eso se hace desde dentro del Control Center. Idempotente: si la version ya
# esta instalada no se vuelve a descargar, solo se reactiva.

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
install-release.sh — Instala el Control Center de GekkoApp (GUI + CLI) desde
el release verificado (manifiesto + SHA-256) de GitHub, sin compilar.

  --version <vX.Y.Z>   Instala una version concreta (por defecto: ultima).
  --prefix <dir>       Prefijo de instalacion (default: $HOME/.local).
  --no-launch          No abrir el Control Center al terminar.
  --uninstall          Desinstala la version instalada.
  --help               Muestra esta ayuda.

Seguridad: solo HTTPS, verificacion del SHA-256 del artefacto contra su
manifiesto y comprobacion de la glibc minima antes de tocar el sistema.
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
# Estado y cache del motor de GekkoApp (installer.rs: InstallPaths::detect).
# Cuando el Control Center se actualiza a si mismo registra `modules.gekkoapp`
# en el estado y deja el artefacto descargado en la cache; la desinstalacion
# limpia esa entrada para que una reinstalacion posterior parta de cero.
STATE_FILE="${XDG_STATE_HOME:-$HOME_DIR/.local/state}/gekkoapp/installations-v1.json"
ARTIFACTS_DIR="${XDG_CACHE_HOME:-$HOME_DIR/.cache}/gekkoapp/artifacts"
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
CLI_NAME="gekkoapp"
# El manifiesto declara los dos entrypoints (bin/gekkoapp y bin/gekkoapp-gui):
# se enlazan ambos, igual que hace la activacion nativa de installer.rs.
ENTRYPOINTS=("$GUI_NAME" "$CLI_NAME")

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

# ¿Es la version A menor que la B? (orden numerico por componentes, como
# `sort -V`; sirve para comparar glibc 2.39 con 2.44).
version_lt() {
  [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | head -n1)" = "$1" ]
}

# ---------------------------------------------------------------------------
# Propiedad de los archivos en $BIN_HOME
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

# ¿Es este archivo regular un binario de GekkoApp dejado por scripts/install.sh?
# Se exige que sea un ELF y que contenga dos cadenas que llevan los dos binarios
# (CLI y GUI) incluso compilados con LTO y strip: el nombre del archivo de
# estado del motor (`installations-v1.json`, installer.rs) y el del producto.
# Cualquier otra cosa con ese nombre se considera ajena y no se toca.
owned_regular() {
  local file="$1"
  [ -f "$file" ] && [ ! -L "$file" ] || return 1
  [ "$(head -c 4 "$file" 2>/dev/null | tr -d '\0')" = $'\x7fELF' ] || return 1
  grep -q -a -F 'installations-v1.json' "$file" 2>/dev/null \
    && grep -q -a -F 'GekkoApp' "$file" 2>/dev/null
}

# ---------------------------------------------------------------------------
# Desinstalar
# ---------------------------------------------------------------------------
uninstall() {
  # Se compara contra $PRODUCT_HOME entero: adivinar la "version activa" por
  # orden lexicografico dejaba el enlace sin borrar y colgando tras el rm -rf.
  local name link
  for name in "${ENTRYPOINTS[@]}"; do
    link="$BIN_HOME/$name"
    if owned_link "$link"; then
      rm -f "$link"
      info "eliminado $link"
    elif [ -L "$link" ]; then
      say "  aviso: $link no apunta a GekkoApp; no se toca"
    elif owned_regular "$link"; then
      # Binario copiado por scripts/install.sh (instalacion desde el fuente).
      rm -f "$link"
      info "eliminado $link (binario instalado desde el codigo fuente)"
    elif [ -e "$link" ]; then
      say "  aviso: $link no es un binario de GekkoApp; no se toca"
    fi
  done
  [ -f "$DESKTOP_FILE" ] && rm -f "$DESKTOP_FILE" && info "eliminado $DESKTOP_FILE"
  # Nombre que usaban versiones antiguas de scripts/install.sh.
  [ -f "$APPS_DIR/gekkoapp-control-center.desktop" ] && rm -f "$APPS_DIR/gekkoapp-control-center.desktop" \
    && info "eliminado $APPS_DIR/gekkoapp-control-center.desktop"
  [ -f "$ICON_DIR/$APP_ID.png" ] && rm -f "$ICON_DIR/$APP_ID.png" && info "eliminado $ICON_DIR/$APP_ID.png"
  [ -f "$SYMBOLIC_DIR/$APP_ID-symbolic.svg" ] && rm -f "$SYMBOLIC_DIR/$APP_ID-symbolic.svg" && info "eliminado $SYMBOLIC_DIR/$APP_ID-symbolic.svg"
  if [ -d "$PRODUCT_HOME" ]; then
    rm -rf "$PRODUCT_HOME"
    info "eliminado $PRODUCT_HOME"
  fi
  forget_engine_state
  # Directorios que creo la instalacion y que quedan vacios. `rmdir` solo
  # borra si no hay nada dentro: si otro producto de kitotsu sigue ahi, se deja.
  local dir
  for dir in "$VERSIONS_HOME" "$HOME_DIR/.local/lib" "$ARTIFACTS_DIR" "${ARTIFACTS_DIR%/*}"; do
    if [ -d "$dir" ] && rmdir "$dir" 2>/dev/null; then
      info "eliminado el directorio vacio $dir"
    fi
  done
  refresh_desktop_db
  say "GekkoApp Control Center desinstalado."
  exit 0
}

# Retira `modules.gekkoapp` del estado del motor (installations-v1.json) sin
# tocar los demas modulos (Kito, Bauh, Gekko ADB); si no queda ninguno se borra
# el fichero. Tambien elimina el artefacto de GekkoApp que el motor cacheo al
# auto-actualizarse, y SOLO ese: se toma su nombre del `artifact_url` registrado
# y se exige que sea `gekkoapp-*.tar.zst` dentro de la cache propia de GekkoApp.
# Sin python3 no se puede editar el JSON con garantias: se avisa y se deja.
forget_engine_state() {
  [ -f "$STATE_FILE" ] || return 0
  if ! command -v python3 >/dev/null 2>&1; then
    say "  aviso: no hay python3; se conserva la entrada gekkoapp de $STATE_FILE"
    return 0
  fi
  local artifact_name
  artifact_name="$(python3 - "$STATE_FILE" "$PRODUCT" <<'PY' 2>/dev/null || true
import json, os, sys

path, product = sys.argv[1], sys.argv[2]
try:
    with open(path, encoding="utf-8") as handle:
        state = json.load(handle)
except (OSError, ValueError):
    sys.exit(1)
modules = state.get("modules")
if not isinstance(modules, dict) or product not in modules:
    sys.exit(0)
module = modules.pop(product)
if modules:
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(state, handle, indent=2, ensure_ascii=False)
        handle.write("\n")
else:
    os.remove(path)
url = module.get("artifact_url") if isinstance(module, dict) else None
if isinstance(url, str) and url:
    print(url.rsplit("/", 1)[-1])
PY
)"
  if [ -f "$STATE_FILE" ]; then
    info "retirada la entrada gekkoapp de $STATE_FILE"
  else
    info "eliminado $STATE_FILE (no quedaban modulos registrados)"
    rmdir "${STATE_FILE%/*}" 2>/dev/null && info "eliminado el directorio vacio ${STATE_FILE%/*}"
  fi
  case "$artifact_name" in
    "$PRODUCT"-*.tar.zst)
      if [ -f "$ARTIFACTS_DIR/$artifact_name" ]; then
        rm -f "$ARTIFACTS_DIR/$artifact_name"
        info "eliminado $ARTIFACTS_DIR/$artifact_name"
      fi
      ;;
    "") ;;
    *) say "  aviso: el artefacto registrado ($artifact_name) no tiene el nombre de un release de GekkoApp; no se toca" ;;
  esac
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
require getconf

if [ -n "$VERSION" ]; then
  TAG="${VERSION#v}"
  info "Usando la version solicitada: v$TAG"
else
  info "Resolviendo el ultimo release de $REPO..."
  TAG="$(resolve_latest_tag)"
  TAG="${TAG#v}"
fi
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

info "Descargando manifiesto..."
curl -fsSL "$MANIFEST_URL" -o "$TMP/manifest.json" \
  || fail "no se pudo descargar el manifiesto $MANIFEST_URL"

manifest_field() {
  python3 -c 'import json,sys
value = json.load(open(sys.argv[1]))
for key in sys.argv[2:]:
    value = value[key]
print(value)' "$TMP/manifest.json" "$@"
}

VERSION_FIELD="$(manifest_field product version)"
ARCHIVE_NAME="$(manifest_field artifact file_name)"
ARCHIVE_SIZE="$(manifest_field artifact size_bytes)"
ARCHIVE_SHA="$(manifest_field artifact sha256)"
GLIBC_MINIMUM="$(manifest_field platform libc minimum 2>/dev/null || true)"

# Los campos del manifiesto se usan como nombre de archivo y de directorio:
# se exige que sean nombres simples, sin separadores de ruta.
for field in "$VERSION_FIELD" "$ARCHIVE_NAME"; do
  case "$field" in
    ""|*/*|.|..) fail "el manifiesto declara un valor no valido: '$field'" ;;
  esac
done

# Puerta de glibc: la misma que aplica installer.rs. Un binario enlazado contra
# una glibc mas nueva que la del host no arranca (simbolos GLIBC_x.y ausentes),
# asi que se aborta antes de descargar nada.
case "$GLIBC_MINIMUM" in
  ""|None) fail "el manifiesto no declara la glibc minima (platform.libc.minimum)" ;;
esac
HOST_GLIBC="$(getconf GNU_LIBC_VERSION 2>/dev/null | awk '{print $2}')"
[ -n "$HOST_GLIBC" ] || fail "no se pudo determinar la glibc del sistema (getconf GNU_LIBC_VERSION)"
if version_lt "$HOST_GLIBC" "$GLIBC_MINIMUM"; then
  fail "GekkoApp $VERSION_FIELD requiere glibc >= $GLIBC_MINIMUM y este sistema tiene glibc $HOST_GLIBC. Actualiza el sistema o usa una version anterior con --version."
fi
info "glibc del sistema: $HOST_GLIBC (minima requerida: $GLIBC_MINIMUM)"

info "Version: $VERSION_FIELD"
FINAL_ROOT="$PRODUCT_HOME/$VERSION_FIELD"
if [ -x "$FINAL_ROOT/bin/$GUI_NAME" ]; then
  # Ya se descargo y verifico en una instalacion anterior: solo se reactiva.
  info "La version $VERSION_FIELD ya esta instalada en $FINAL_ROOT; no se vuelve a descargar."
else
  info "Descargando artefacto ($ARCHIVE_NAME)..."
  curl -fsSL "$RELEASE_BASE/$ARCHIVE_NAME" -o "$TMP/$ARCHIVE_NAME" \
    || fail "no se pudo descargar el artefacto"

  ACTUAL_SIZE="$(stat -c %s "$TMP/$ARCHIVE_NAME")"
  ACTUAL_SHA="$(sha256sum "$TMP/$ARCHIVE_NAME" | awk '{print $1}')"
  [ "$ACTUAL_SIZE" = "$ARCHIVE_SIZE" ] || fail "el tamano del artefacto no coincide con el manifiesto"
  [ "$ACTUAL_SHA" = "$ARCHIVE_SHA" ] || fail "la verificacion SHA-256 del artefacto fallo; se aborta por seguridad"
  info "Verificacion SHA-256 correcta."

  # Un directorio a medias (instalacion interrumpida) no vale como instalado.
  rm -rf "$FINAL_ROOT"
  info "Extrayendo a $FINAL_ROOT ..."
  mkdir -p "$PRODUCT_HOME"
  tar --zstd -xf "$TMP/$ARCHIVE_NAME" -C "$TMP"
  EXTRACTED="$(find "$TMP" -mindepth 1 -maxdepth 1 -type d | head -n1)"
  mv "$EXTRACTED" "$FINAL_ROOT"
fi

for name in "${ENTRYPOINTS[@]}"; do
  [ -x "$FINAL_ROOT/bin/$name" ] || fail "el release no contiene bin/$name"
done

mkdir -p "$BIN_HOME" "$APPS_DIR" "$ICON_DIR" "$SYMBOLIC_DIR"

# Entrypoints: symlinks propios en ~/.local/bin (no se pisa una ruta ajena).
# Un enlace que apunte a CUALQUIER version bajo $PRODUCT_HOME es nuestro: antes
# se exigia que apuntase ya a la version nueva, asi que toda actualizacion
# terminaba en "no se sobreescribira una ruta ajena".
activate_link() {
  local name="$1" link desired current
  link="$BIN_HOME/$name"
  desired="$(readlink -m "$FINAL_ROOT/bin/$name")"
  if [ -L "$link" ]; then
    # `-m` resuelve tambien los enlaces rotos: con `-f`, un enlace a una version
    # ya borrada devolvia vacio y abortaba la instalacion como "ruta ajena".
    current="$(readlink -m "$link" 2>/dev/null || true)"
    if [ "$current" != "$desired" ] \
       && [ "${current#"$PRODUCT_HOME"/}" = "$current" ]; then
      fail "no se sobreescribira una ruta ajena: $link -> ${current:-?}"
    fi
  elif owned_regular "$link"; then
    # Instalacion previa desde el codigo fuente (scripts/install.sh copia un
    # binario regular). Es nuestra: se adopta, igual que hace installer.rs.
    info "Reemplazando el $name instalado desde el codigo fuente."
  elif [ -e "$link" ]; then
    fail "no se sobreescribira una ruta ajena: $link no es un binario de GekkoApp"
  fi
  ln -sfn "$FINAL_ROOT/bin/$name" "$link"
  info "Instalado: $link"
}

for name in "${ENTRYPOINTS[@]}"; do
  activate_link "$name"
done
GUI_LINK="$BIN_HOME/$GUI_NAME"

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
say "  CLI:    $BIN_HOME/$CLI_NAME"
say "  Menu:   $DESKTOP_FILE"
say "  Desde el Control Center podras instalar y actualizar todo lo demas."
say ""

if [ "$LAUNCH" = "1" ]; then
  info "Abriendo el Control Center..."
  nohup "$GUI_LINK" </dev/null >/dev/null 2>&1 &
  disown
fi
