#!/usr/bin/env bash
#
# build-release-bundle.sh — Empaqueta un tarball de release de GekkoApp.
#
# Uso:
#   ./scripts/build-release-bundle.sh [DIST_DIR]
#
# Requiere: cargo, tar (zstd), los binarios release ya compilados
# (cargo build --release + cargo build --release --features gui).
#
# Genera en DIST_DIR (default: releases/dist):
#   gekkoapp-<version>.tar.zst
#   gekkoapp-<version>.sha256
#   gekkoapp-<version>.manifest.json   (contrato kitotsu.release-artifact 1.0)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CRATE_DIR="$ROOT_DIR/Gekko APP/gekkoapp-rs"

DIST_DIR="${1:-$ROOT_DIR/releases/dist}"
mkdir -p "$DIST_DIR"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$CRATE_DIR/Cargo.toml" | head -n1)"
[ -n "$VERSION" ] || { echo "error: no se pudo leer la version del crate" >&2; exit 1; }
TAG="v$VERSION"
PRODUCT_ID="gekkoapp"
REPOSITORY="The-Gekko/GekkoApp"
TARGET="x86_64-unknown-linux-gnu"
APP_ID="org.thegekko.gekkoapp"

for bin in "$CRATE_DIR/target/release/gekkoapp" "$CRATE_DIR/target/release/gekkoapp-gui"; do
  [ -x "$bin" ] || { echo "error: falta $bin (compila primero)" >&2; exit 1; }
done

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
ROOT="$PRODUCT_ID-$VERSION"
mkdir -p "$STAGE/$ROOT/bin"

install -m 0755 "$CRATE_DIR/target/release/gekkoapp" "$STAGE/$ROOT/bin/gekkoapp"
install -m 0755 "$CRATE_DIR/target/release/gekkoapp-gui" "$STAGE/$ROOT/bin/gekkoapp-gui"
install -m 0755 "$ROOT_DIR/GekkoApp.sh" "$STAGE/$ROOT/gekkoapp.sh"
install -m 0644 "$ROOT_DIR/packaging/gekkoapp-control-center.desktop" "$STAGE/$ROOT/gekkoapp-control-center.desktop"
install -m 0644 "$CRATE_DIR/icons/icon.png" "$STAGE/$ROOT/$APP_ID.png"
install -m 0644 "$ROOT_DIR/README.md" "$STAGE/$ROOT/README.md"
install -m 0644 "$ROOT_DIR/LICENSE" "$STAGE/$ROOT/LICENSE"

# Tokeniza el Exec de la entrada .desktop para que el usuario pueda reemplazarlo.
sed -i "s|^Exec=.*|Exec=/usr/bin/gekkoapp-gui|" "$STAGE/$ROOT/gekkoapp-control-center.desktop"

ARCHIVE="$PRODUCT_ID-$VERSION.tar.zst"
tar --zstd -C "$STAGE" -cf "$DIST_DIR/$ARCHIVE" "$ROOT"
ARCHIVE_SIZE="$(stat -c %s "$DIST_DIR/$ARCHIVE")"
ARCHIVE_SHA256="$(sha256sum "$DIST_DIR/$ARCHIVE" | awk '{print $1}')"
printf '%s  %s\n' "$ARCHIVE_SHA256" "$ARCHIVE" > "$DIST_DIR/$PRODUCT_ID-$VERSION.sha256"

# Genera el manifest sobre el arbol real extraido.
EXTRACT="$(mktemp -d)"
tar --zstd -xf "$DIST_DIR/$ARCHIVE" -C "$EXTRACT"
python3 - "$EXTRACT" "$DIST_DIR" "$ARCHIVE" "$TAG" "$VERSION" "$REPOSITORY" "$TARGET" "$APP_ID" "$ARCHIVE_SIZE" "$ARCHIVE_SHA256" "$ROOT" <<'PYEOF'
import hashlib, json, os, stat, sys

extract, dist_dir, archive = sys.argv[1], sys.argv[2], sys.argv[3]
tag, version = sys.argv[4], sys.argv[5]
repository, target = sys.argv[6], sys.argv[7]
app_id, archive_size, archive_sha256 = sys.argv[8], int(sys.argv[9]), sys.argv[10]
root = sys.argv[11]

tree = os.path.join(extract, root)
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
        elif rel.endswith(".desktop"):
            kind = "desktop-entry"
        elif rel.endswith(".png"):
            kind = "icon"
        else:
            kind = "resource"
        with open(full, "rb") as fh:
            digest = hashlib.sha256(fh.read()).hexdigest()
        payload.append({
            "path": rel, "kind": kind, "mode": "0%o" % mode,
            "size_bytes": st.st_size, "sha256": digest,
        })

manifest = {
    "schema_version": 1,
    "kind": "kitotsu.release-artifact",
    "distribution_contract": "1.0",
    "install_method": "binary_extract",
    "product": {
        "id": "gekkoapp", "version": version, "repository": repository,
        "contract_version": "1.0",
    },
    "release": {"tag": tag, "channel": "stable"},
    "platform": {
        "os": "linux", "arch": "x86_64", "target": target,
        "libc": {"family": "glibc", "minimum": "2.34"},
    },
    "artifact": {
        "file_name": archive, "format": "tar.zst",
        "size_bytes": archive_size, "sha256": archive_sha256,
    },
    "payload": payload,
    "entrypoints": [
        {"name": "gekkoapp", "path": "bin/gekkoapp"},
        {"name": "gekkoapp-gui", "path": "bin/gekkoapp-gui"},
    ],
    "requirements": {"modules": [], "host_capabilities": []},
    "integrations": {
        "desktop_entries": [
            {
                "application_id": app_id,
                "template": "gekkoapp-control-center.desktop",
                "entrypoint": "gekkoapp-gui",
                "icons": [
                    {"source": "%s.png" % app_id, "theme": "hicolor", "size": 512, "format": "png"}
                ],
            }
        ]
    },
}

with open(os.path.join(dist_dir, "gekkoapp-%s.manifest.json" % version), "w", encoding="utf-8") as fh:
    json.dump(manifest, fh, indent=2, ensure_ascii=False)
    fh.write("\n")
print("payload_files=%d" % len(payload))
PYEOF
rm -rf "$EXTRACT"

echo "==> Listo"
echo "  Artefacto:  $DIST_DIR/$ARCHIVE"
echo "  SHA256:     $DIST_DIR/$PRODUCT_ID-$VERSION.sha256"
echo "  Manifest:   $DIST_DIR/$PRODUCT_ID-$VERSION.manifest.json"
echo
echo "Para publicar: gh release create '$TAG' '$DIST_DIR/$ARCHIVE' '$DIST_DIR/$PRODUCT_ID-$VERSION.sha256' '$DIST_DIR/$PRODUCT_ID-$VERSION.manifest.json' --repo '$REPOSITORY' --title 'GekkoApp $VERSION'"
