#!/usr/bin/env bash
#
# build-bauh-release.sh — Empaqueta un release firmado de Bauh Fork (The-Gekko)
# compatible con el motor de GekkoApp (contrato "kitotsu.release-artifact" 1.0,
# install_method "python_pipx").
#
# Uso:
#   ./build-bauh-release.sh [VERSION] [TARGET] [DIST_DIR]
#
#   VERSION   version del release (ej. 0.10.7). Por defecto lee bauh/__init__.py.
#   TARGET    target de release (ej. x86_64-unknown-linux-gnu). Default: x86_64-unknown-linux-gnu.
#   DIST_DIR  directorio de salida (archivo .tar.zst + <product>-<target>.manifest.json).
#            Default: $BAUH_REPO_ROOT/releases/dist
#
# El script copia SOLO lo necesario para que `pipx install` construya el
# paquete, genera la plantilla .desktop y el icono PNG (contrato hicolor), y
# calcula payload + hashes del artefacto.
#
# Requisitos: tar (zstd), python3, rsvg-convert (o convert/magick).

set -euo pipefail

BAUH_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Permite apuntar al checkout del fork.
BAUH_SRC="${BAUH_SRC:-$BAUH_REPO_ROOT}"
BAUH_CHECKOUT="$BAUH_SRC"
# Verifica que el checkout sea el fork correcto.
if [ ! -f "$BAUH_SRC/bauh/__init__.py" ]; then
  echo "error: $BAUH_SRC no parece ser el checkout del fork de Bauh (falta bauh/__init__.py)" >&2
  exit 1
fi

VERSION="${1:-$(sed -n "s/^__version__ = ['\"]\([^'\"]*\)['\"]/\1/p" "$BAUH_SRC/bauh/__init__.py" | head -n1)}"
TARGET="${2:-x86_64-unknown-linux-gnu}"
DIST_DIR="${3:-$BAUH_REPO_ROOT/releases/dist}"

PRODUCT_ID="bauh-fork-the-gekko"
REPOSITORY="The-Gekko/Bauh-Fork-The-Gekko"
APP_ID="org.thegekko.bauh"
GLIBC_MINIMUM="${GLIBC_MINIMUM:-2.34}"
ICON_SIZE=512

if [ -z "$VERSION" ]; then
  echo "error: no se pudo determinar la version (usa el argumento VERSION)" >&2
  exit 1
fi
if [[ "$VERSION" != v* ]]; then
  TAG="v$VERSION"
else
  TAG="$VERSION"
  VERSION="${VERSION#v}"
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

ROOT="$PRODUCT_ID-$VERSION"
ARCHIVE="$PRODUCT_ID-$VERSION.tar.zst"
MANIFEST_NAME="$PRODUCT_ID-$TARGET.manifest.json"

echo "==> Preparando arbol fuente en $STAGE/$ROOT"
mkdir -p "$STAGE/$ROOT"

# Copia lo imprescindible para que pipx pueda construir el paquete.
cp -a "$BAUH_SRC/bauh" "$STAGE/$ROOT/"
find "$STAGE/$ROOT" -name __pycache__ -type d -prune -exec rm -rf {} +
rm -rf "$STAGE/$ROOT/bauh.egg-info" "$STAGE/$ROOT/build"
for file in setup.py setup.cfg pyproject.toml requirements.txt MANIFEST.in README.md CHANGELOG.md LICENSE CREDITS.md CONTRIBUTING.md; do
  [ -e "$BAUH_SRC/$file" ] && cp -a "$BAUH_SRC/$file" "$STAGE/$ROOT/"
done

# Plantilla .desktop: tokeniza Exec y apunta Icono al application id.
DESKTOP_SRC="$BAUH_SRC/bauh/desktop/bauh.desktop"
DESKTOP_TEMPLATE="$STAGE/$ROOT/bauh/desktop/bauh.desktop.template"
if [ ! -f "$DESKTOP_SRC" ]; then
  echo "error: falta $DESKTOP_SRC" >&2
  exit 1
fi
sed -e 's|^Exec=.*|Exec=@EXECUTABLE@|' \
    -e "s|^Icon=.*|Icon=$APP_ID|" \
    "$DESKTOP_SRC" > "$DESKTOP_TEMPLATE"
if grep -q '@' "$DESKTOP_TEMPLATE" && [ "$(grep -o '@' "$DESKTOP_TEMPLATE" | wc -l)" != "2" ]; then
  echo "error: la plantilla .desktop contiene tokens '@' no admitidos" >&2
  exit 1
fi

# Icono PNG hicolor a partir del logo SVG.
ICON_SOURCE="$STAGE/$ROOT/bauh/desktop/$APP_ID.png"
if command -v rsvg-convert >/dev/null 2>&1; then
  rsvg-convert -w "$ICON_SIZE" -h "$ICON_SIZE" -o "$ICON_SOURCE" "$BAUH_SRC/bauh/view/resources/img/logo.svg"
elif command -v magick >/dev/null 2>&1; then
  magick -background none -density 96 "$BAUH_SRC/bauh/view/resources/img/logo.svg" -resize "${ICON_SIZE}x${ICON_SIZE}" "$ICON_SOURCE"
elif command -v convert >/dev/null 2>&1; then
  convert -background none -density 96 "$BAUH_SRC/bauh/view/resources/img/logo.svg" -resize "${ICON_SIZE}x${ICON_SIZE}" "$ICON_SOURCE"
else
  echo "error: falta rsvg-convert, magick o convert para generar el icono PNG" >&2
  exit 1
fi

echo "==> Creando archivo fuente ($ARCHIVE)"
mkdir -p "$DIST_DIR"
tar --zstd -C "$STAGE" -cf "$DIST_DIR/$ARCHIVE" "$ROOT"
ARCHIVE_SIZE="$(stat -c %s "$DIST_DIR/$ARCHIVE")"
ARCHIVE_SHA256="$(sha256sum "$DIST_DIR/$ARCHIVE" | awk '{print $1}')"

echo "==> Calculando payload"
MANIFEST="$(python3 - "$STAGE" "$ROOT" "$DIST_DIR" "$ARCHIVE" "$TAG" "$TARGET" "$PRODUCT_ID" "$REPOSITORY" "$APP_ID" "$GLIBC_MINIMUM" "$VERSION" "$ARCHIVE_SIZE" "$ARCHIVE_SHA256" "$MANIFEST_NAME" <<'PYEOF'
import hashlib, json, os, stat, sys

stage, root, dist_dir, archive = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
tag, target = sys.argv[5], sys.argv[6]
product_id, repository, app_id = sys.argv[7], sys.argv[8], sys.argv[9]
glibc_min, version = sys.argv[10], sys.argv[11]
archive_size, archive_sha256 = int(sys.argv[12]), sys.argv[13]
manifest_name = sys.argv[14]

tree = os.path.join(stage, root)
payload = []
for dirpath, dirnames, filenames in os.walk(tree):
    for name in sorted(filenames):
        full = os.path.join(dirpath, name)
        rel = os.path.relpath(full, tree)
        st = os.stat(full)
        mode = stat.S_IMODE(st.st_mode)
        if mode & 0o111:
            kind = "executable"
        elif rel == "LICENSE":
            kind = "license"
        elif rel.endswith(".desktop.template"):
            kind = "desktop-entry-template"
        elif rel.endswith(".png"):
            kind = "icon"
        else:
            kind = "resource"
        with open(full, "rb") as fh:
            digest = hashlib.sha256(fh.read()).hexdigest()
        payload.append({
            "path": rel,
            "kind": kind,
            "mode": "0%o" % mode,
            "size_bytes": st.st_size,
            "sha256": digest,
        })

manifest = {
    "schema_version": 1,
    "kind": "kitotsu.release-artifact",
    "distribution_contract": "1.0",
    "install_method": "python_pipx",
    "product": {
        "id": product_id,
        "version": version,
        "repository": repository,
        "contract_version": "1.0",
    },
    "release": {"tag": tag, "channel": "stable"},
    "platform": {
        "os": "linux",
        "arch": "x86_64",
        "target": target,
        "libc": {"family": "glibc", "minimum": glibc_min},
    },
    "artifact": {
        "file_name": archive,
        "format": "tar.zst",
        "size_bytes": archive_size,
        "sha256": archive_sha256,
    },
    "payload": payload,
    "entrypoints": [
        {"name": "bauh", "path": "bauh/app.py"},
        {"name": "bauh-tray", "path": "bauh/app.py"},
        {"name": "bauh-cli", "path": "bauh/cli/app.py"},
    ],
    "requirements": {"modules": [], "host_capabilities": []},
    "integrations": {
        "desktop_entries": [
            {
                "application_id": app_id,
                "template": "bauh/desktop/bauh.desktop.template",
                "entrypoint": "bauh",
                "icons": [
                    {
                        "source": "bauh/desktop/%s.png" % app_id,
                        "theme": "hicolor",
                        "size": 512,
                        "format": "png",
                    }
                ],
            }
        ]
    },
}

with open(os.path.join(dist_dir, manifest_name), "w", encoding="utf-8") as fh:
    json.dump(manifest, fh, indent=2, ensure_ascii=False)
    fh.write("\n")
print("payload_files=%d" % len(payload))
PYEOF
)"

echo "$MANIFEST"
echo "==> Listo"
echo "  Artefacto:  $DIST_DIR/$ARCHIVE"
echo "  Manifiesto: $DIST_DIR/$MANIFEST_NAME"
echo "  Tag:        $TAG"
echo
echo "Para publicar (requiere gh autenticado):"
echo "  cd '$BAUH_CHECKOUT' && git tag '$TAG' && git push origin '$TAG'"
echo "  gh release create '$TAG' '$DIST_DIR/$ARCHIVE' '$DIST_DIR/$MANIFEST_NAME' --repo '$REPOSITORY' --title 'Bauh Fork $VERSION'"
