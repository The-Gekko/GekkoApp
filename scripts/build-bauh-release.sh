#!/usr/bin/env bash
#
# build-bauh-release.sh — Empaqueta un release verificable (manifiesto + SHA-256) de Bauh Fork (The-Gekko)
# compatible con el motor de GekkoApp (contrato "kitotsu.release-artifact" 1.0,
# install_method "python_pipx").
#
# Uso:
#   ./build-bauh-release.sh [VERSION] [TARGET] [DIST_DIR]
#
#   VERSION   version del release (ej. 0.10.8+gekko.1 o la etiqueta v0.10.8-gekko.1).
#             Por defecto lee bauh.__version__ de bauh/__init__.py.
#   TARGET    target de release (ej. x86_64-unknown-linux-gnu). Default: x86_64-unknown-linux-gnu.
#   DIST_DIR  directorio de salida (archivo .tar.zst + <product>-<target>.manifest.json).
#            Default: $BAUH_REPO_ROOT/releases/dist
#
# Esquema de versiones del fork (docs/DISTRIBUCION.md del fork):
#   bauh.__version__      X.Y.Z+gekko.N   (PEP 440, local version)  -> product.version
#   etiqueta git          vX.Y.Z-gekko.N  (con GUION)               -> release.tag
#   nombre del artefacto  bauh-fork-the-gekko-X.Y.Z.gekko.N.tar.zst (el '+' se
#                         sustituye por '.', porque GitHub renombra los assets
#                         con caracteres especiales)
# GekkoApp (installer.rs validate_manifest) acepta que release.tag sea
# 'v'+version o 'v'+version con '+' -> '-'.
#
# El script copia SOLO lo necesario para que `pipx install` construya el
# paquete, genera las plantillas .desktop (app y bandeja) y el icono PNG
# (contrato hicolor), y calcula payload + hashes del artefacto.
#
# Este mismo script esta vendorizado en el fork como
# tools/build-gekkoapp-release.sh (lo ejecuta su release.yml en cada etiqueta
# v*): los dos deben mantenerse funcionalmente equivalentes.
#
# Requisitos: tar (zstd), python3, rsvg-convert (solo si falta el PNG de 512).

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
# El repositorio se renombro a The-Gekko-Bauh; el PRODUCT_ID se mantiene
# porque es el prefijo de los artefactos ya publicados.
REPOSITORY="The-Gekko/The-Gekko-Bauh"
APP_ID="org.thegekko.bauh"
# Entrada de menu de la bandeja (gekko-bauh-tray). validate_application_id exige
# un id inverso-DNS de al menos tres segmentos; cuatro son validos.
TRAY_APP_ID="org.thegekko.bauh.tray"
# Bauh es Python puro: la glibc no la impone el artefacto, pero el contrato
# exige declarar una minima. 2.34 cubre cualquier Arch/Solus soportado.
GLIBC_MINIMUM="${GLIBC_MINIMUM:-2.34}"
ICON_SIZE=512

if [ -z "$VERSION" ]; then
  echo "error: no se pudo determinar la version (usa el argumento VERSION)" >&2
  exit 1
fi
# La etiqueta git se escribe con guion (v0.10.8-gekko.1) y la version PEP 440
# con '+' (0.10.8+gekko.1): la conversion es un unico cambio de caracter en
# cada sentido. Si se recibe la etiqueta, se deshace para obtener la version.
if [[ "$VERSION" == v* ]]; then
  TAG="$VERSION"
  VERSION="${VERSION#v}"
  VERSION="${VERSION/-/+}"
else
  TAG="v${VERSION/+/-}"
fi
case "$VERSION" in
  *[!A-Za-z0-9.+]*)
    echo "error: version no valida (se espera X.Y.Z o X.Y.Z+gekko.N): $VERSION" >&2
    exit 1 ;;
esac

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

ROOT="$PRODUCT_ID-$VERSION"
# Nombre del artefacto SIN caracteres especiales. GitHub renombra los assets
# que los llevan (docs de la API: "GitHub renames asset filenames that have
# special characters, non-alphanumeric characters, and leading or trailing
# periods") y GekkoApp localiza el asset por el nombre EXACTO que declara el
# manifiesto: con el `+` de una version local PEP 440 (0.10.8+gekko.1) el
# release publicado quedaria irresoluble. Solo cambia el nombre del archivo:
# product.version conserva el `+` y release.tag lleva el guion de la etiqueta.
ARCHIVE_VERSION="${VERSION//+/.}"
ARCHIVE="$PRODUCT_ID-$ARCHIVE_VERSION.tar.zst"
case "$ARCHIVE" in
  *[!A-Za-z0-9._-]*)
    echo "error: el nombre del artefacto contiene caracteres que GitHub renombraria: $ARCHIVE" >&2
    exit 1 ;;
esac
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
# El fork renombro su lanzador a gekko-bauh.desktop para no tapar por
# precedencia XDG al paquete oficial; se acepta el nombre antiguo por si se
# empaqueta un checkout viejo.
DESKTOP_SRC=""
for candidate in "$BAUH_SRC/bauh/desktop/gekko-bauh.desktop" "$BAUH_SRC/bauh/desktop/bauh.desktop"; do
  if [ -f "$candidate" ]; then
    DESKTOP_SRC="$candidate"
    break
  fi
done
DESKTOP_TEMPLATE="$STAGE/$ROOT/bauh/desktop/bauh.desktop.template"
if [ -z "$DESKTOP_SRC" ]; then
  echo "error: no se encontro bauh/desktop/gekko-bauh.desktop en $BAUH_SRC" >&2
  exit 1
fi
sed -e 's|^Exec=.*|Exec=@EXECUTABLE@|' \
    -e "s|^Icon=.*|Icon=$APP_ID|" \
    "$DESKTOP_SRC" > "$DESKTOP_TEMPLATE"
if grep -q '@' "$DESKTOP_TEMPLATE" && [ "$(grep -o '@' "$DESKTOP_TEMPLATE" | wc -l)" != "2" ]; then
  echo "error: la plantilla .desktop contiene tokens '@' no admitidos" >&2
  exit 1
fi

# Segunda entrada de menu: la bandeja (gekko-bauh-tray.desktop, entrypoint
# gekko-bauh-tray). Es opcional: un checkout antiguo sin ella sigue
# empaquetandose con una sola entrada. El motor instala el icono de cada
# entrada como <application_id>.png, asi que Icon= apunta al id de la bandeja.
TRAY_DESKTOP_SRC="$BAUH_SRC/bauh/desktop/gekko-bauh-tray.desktop"
TRAY_TEMPLATE_REL=""
if [ -f "$TRAY_DESKTOP_SRC" ]; then
  TRAY_TEMPLATE_REL="bauh/desktop/bauh-tray.desktop.template"
  sed -e 's|^Exec=.*|Exec=@EXECUTABLE@|' \
      -e "s|^Icon=.*|Icon=$TRAY_APP_ID|" \
      "$TRAY_DESKTOP_SRC" > "$STAGE/$ROOT/$TRAY_TEMPLATE_REL"
  if grep -q '@' "$STAGE/$ROOT/$TRAY_TEMPLATE_REL" && [ "$(grep -o '@' "$STAGE/$ROOT/$TRAY_TEMPLATE_REL" | wc -l)" != "2" ]; then
    echo "error: la plantilla .desktop de la bandeja contiene tokens '@' no admitidos" >&2
    exit 1
  fi
fi

# Icono PNG hicolor. El fork ya publica los PNG por tamano en pictures/icons,
# asi que se copia el de 512 en vez de rasterizar un SVG (bauh ya no distribuye
# view/resources/img/logo.svg).
ICON_SOURCE="$STAGE/$ROOT/bauh/desktop/$APP_ID.png"
# Se exige el PNG del tamano exacto que se va a declarar en el manifiesto. No
# vale un fallback de otro tamano: el manifiesto dice "size": 512 y el motor lo
# instala en hicolor/512x512, asi que publicar ahi un PNG de 256 daria un icono
# borroso o mal escalado en el menu.
ICON_ORIGIN="$BAUH_SRC/pictures/icons/gekko-bauh-$ICON_SIZE.png"
if [ -f "$ICON_ORIGIN" ]; then
  install -m 0644 "$ICON_ORIGIN" "$ICON_SOURCE"
elif [ -f "$BAUH_SRC/bauh/view/resources/img/logo.svg" ] && command -v rsvg-convert >/dev/null 2>&1; then
  rsvg-convert -w "$ICON_SIZE" -h "$ICON_SIZE" -o "$ICON_SOURCE" "$BAUH_SRC/bauh/view/resources/img/logo.svg"
else
  echo "error: falta el icono de $ICON_SIZE px: $ICON_ORIGIN" >&2
  exit 1
fi

echo "==> Creando archivo fuente ($ARCHIVE)"
mkdir -p "$DIST_DIR"
tar --zstd -C "$STAGE" -cf "$DIST_DIR/$ARCHIVE" "$ROOT"
ARCHIVE_SIZE="$(stat -c %s "$DIST_DIR/$ARCHIVE")"
ARCHIVE_SHA256="$(sha256sum "$DIST_DIR/$ARCHIVE" | awk '{print $1}')"

echo "==> Calculando payload"
MANIFEST="$(python3 - "$STAGE" "$ROOT" "$DIST_DIR" "$ARCHIVE" "$TAG" "$TARGET" "$PRODUCT_ID" "$REPOSITORY" "$APP_ID" "$GLIBC_MINIMUM" "$VERSION" "$ARCHIVE_SIZE" "$ARCHIVE_SHA256" "$MANIFEST_NAME" "$TRAY_APP_ID" "$TRAY_TEMPLATE_REL" <<'PYEOF'
import hashlib, json, os, stat, sys

stage, root, dist_dir, archive = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
tag, target = sys.argv[5], sys.argv[6]
product_id, repository, app_id = sys.argv[7], sys.argv[8], sys.argv[9]
glibc_min, version = sys.argv[10], sys.argv[11]
archive_size, archive_sha256 = int(sys.argv[12]), sys.argv[13]
manifest_name = sys.argv[14]
tray_app_id, tray_template = sys.argv[15], sys.argv[16]

# Coherencia version <-> etiqueta, la misma regla que installer.rs
# (validate_manifest): 'v' + version, o 'v' + version con '+' -> '-'.
if tag not in ("v" + version, "v" + version.replace("+", "-")):
    sys.exit("error: la etiqueta %s no corresponde a la version %s" % (tag, version))

tree = os.path.join(stage, root)


def parse_pyproject(path):
    """Devuelve (nombre de distribucion, {script: destino}) de un pyproject.toml.

    Se lee a mano para no depender de tomllib (Python >= 3.11) en el host que
    empaqueta. pipx registra el entorno con el nombre de la distribucion y crea
    un ejecutable por cada entrada de [project.scripts]: el manifiesto tiene que
    declarar exactamente esos, o la activacion fallara al no encontrarlos.
    """
    name, scripts, section = None, {}, None
    with open(path, encoding="utf-8") as fh:
        for raw in fh:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("[") and line.endswith("]"):
                section = line[1:-1]
                continue
            if "=" not in line:
                continue
            key, value = line.split("=", 1)
            key = key.strip().strip('"').strip("'")
            value = value.split("#")[0].strip().strip('"').strip("'")
            if section == "project" and key == "name" and name is None:
                # Solo el primer `name` de [project]. Una tabla inline
                # multilinea (authors = [\n  { name = "..." },\n]) podria
                # aportar otro `name` y suplantar al de la distribucion.
                name = value
            elif section == "project.scripts" and value:
                scripts[key] = value
    return name, scripts


distribution, scripts = parse_pyproject(os.path.join(tree, "pyproject.toml"))
if not distribution:
    sys.exit("error: pyproject.toml no declara [project] name")
if not scripts:
    sys.exit("error: pyproject.toml no declara [project.scripts]")

entrypoints = []
for script in sorted(scripts):
    module = scripts[script].split(":")[0]
    entrypoints.append({"name": script, "path": module.replace(".", "/") + ".py"})

# El lanzador principal es el que se llama como la distribucion.
primary = distribution if distribution in scripts else sorted(scripts)[0]
# La bandeja solo se declara si el checkout trae su .desktop y el proyecto
# publica el ejecutable correspondiente; si no, se avisa y se omite.
tray_entrypoint = primary + "-tray"
declare_tray = bool(tray_template) and tray_entrypoint in scripts
if tray_template and not declare_tray:
    print("aviso: %s no esta en [project.scripts]; no se declara la entrada de bandeja" % tray_entrypoint, file=sys.stderr)

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
    "entrypoints": entrypoints,
    "pipx_distribution": distribution,
    "requirements": {"modules": [], "host_capabilities": []},
    "integrations": {
        "desktop_entries": [
            {
                "application_id": app_id,
                "template": "bauh/desktop/bauh.desktop.template",
                "entrypoint": primary,
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
if declare_tray:
    # Mismo PNG de origen: el motor lo instala como <tray_app_id>.png.
    manifest["integrations"]["desktop_entries"].append({
        "application_id": tray_app_id,
        "template": tray_template,
        "entrypoint": tray_entrypoint,
        "icons": [
            {
                "source": "bauh/desktop/%s.png" % app_id,
                "theme": "hicolor",
                "size": 512,
                "format": "png",
            }
        ],
    })

declared = {entry["path"] for entry in payload}
missing = [e["path"] for e in entrypoints if e["path"] not in declared]
if missing:
    sys.exit("error: entrypoints fuera del payload: %s" % ", ".join(missing))

with open(os.path.join(dist_dir, manifest_name), "w", encoding="utf-8") as fh:
    json.dump(manifest, fh, indent=2, ensure_ascii=False)
    fh.write("\n")
print("payload_files=%d" % len(payload))
print("pipx_distribution=%s" % distribution)
print("entrypoints=%s" % ", ".join(e["name"] for e in entrypoints))
print("desktop_entries=%s" % ", ".join(d["application_id"] for d in manifest["integrations"]["desktop_entries"]))
PYEOF
)"

echo "$MANIFEST"
echo "==> Listo"
echo "  Artefacto:  $DIST_DIR/$ARCHIVE"
echo "  Manifiesto: $DIST_DIR/$MANIFEST_NAME"
echo "  Version:    $VERSION"
echo "  Tag:        $TAG"
echo
echo "Para publicar: empuja la etiqueta y el release.yml del fork genera y adjunta estos mismos assets:"
echo "  cd '$BAUH_CHECKOUT' && git tag -a '$TAG' -m 'gekko-bauh $VERSION' && git push origin '$TAG'"
echo "A mano (requiere gh autenticado), adjuntandolos a un release ya creado o creandolo:"
echo "  gh release create '$TAG' '$DIST_DIR/$ARCHIVE' '$DIST_DIR/$MANIFEST_NAME' --repo '$REPOSITORY' --title 'bauh Gekko Edition $VERSION'"
