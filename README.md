<p align="center">
  <img src="Gekko%20APP.png" alt="GekkoApp" width="320"/>
  <br/>
  <em>Imagen hecha con IA · Gemini en su modelo Nano Banana</em>
</p>

# 🚀 GekkoApp: Control Center de The-Gekko

**GekkoApp** es un **Control Center de escritorio** (Rust + Tauri v2) para Arch Linux, Garuda y Solus. Con un clic instalas, actualizas, desinstalas y mantienes todo tu entorno sin tocar la terminal.

> Versión: **1.4.0**. Los releases están en [Releases](https://github.com/The-Gekko/GekkoApp/releases); desde la app se actualiza con el botón **Actualizar GekkoApp**.

## ✨ Lo que trae

- 🖥️ **Control Center (GUI)** — desde aquí lo controlas todo: instalar, actualizar, desinstalar componentes y ver novedades con la campana 🔔. Cada acción muestra antes un diálogo de confirmación con lo que va a cambiar y se puede cancelar; los de Bauh y Gekko ADB detallan paquetes y rutas.
- 🐧 **Soporte Multi-Distro (Arch, Garuda y Solus)** — detecta `pacman` y `eopkg` automáticamente y adapta los paquetes y las vistas.
- 🗑️ **Desinstalación limpia e idempotente** — desinstala Kito, Bauh y Gekko ADB desde la GUI o el CLI. Sin residuos: se borra lo que GekkoApp instaló y se conservan tus datos (`~/.config/gekko-bauh`, `~/.config/gekko-adb`, `~/.local/state/gekko-adb/logs`).
- 🦊 **Entorno Kito** — KiUI, Kitsune Compositor y módulos (Kitowall, Kilivepaper, KiSDDM) desde releases verificados (manifiesto + SHA-256).
- 🛍️ **bauh Gekko Edition (Bauh Fork)** — instalación aislada con `pipx` desde un release verificado por SHA-256.
- 📱 **Gekko ADB Studio** — suite GTK de control ADB (scrcpy, shell, debloat y presets), instalada desde el código fuente del repositorio.
- 🔄 **Auto-update de GekkoApp** — la app se actualiza a sí misma desde un release verificado (manifiesto + SHA-256), sin sudo, y al terminar ofrece reiniciarse para abrir la versión nueva.
- 🔔 **Campana de actualizaciones sin depender de GekkoApp** — cada vez que abres el Control Center (y tras cada operación) pregunta a GitHub en ese momento: el último release de Kito, Bauh y GekkoApp, y el último commit de `main` de Gekko ADB Studio frente al que tienes instalado. Una versión nueva de cualquiera de ellos aparece sola: **no hace falta publicar otra GekkoApp para que la vea**.
- 📦 **Chaotic AUR** — repositorios optimizados en un clic (Arch Linux).
- 🎨 **Tema adaptativo** — sigue la paleta de matugen en vivo (Material You).

Los releases de **GekkoApp, Kito y Bauh** se verifican (HTTPS + tamaño + SHA-256 contra su manifiesto) antes de tocar tu sistema. **Gekko ADB Studio es la excepción**: aún no publica releases, así que se clona desde `main` por HTTPS. **Nada de `curl | sh` de terceros.**

> **Retirado en 1.3.0**: Terminal Bonita (ZSH + Starship), los presets Hyprland y Niri y el Gaming Setup ya no forman parte de GekkoApp. Si los instalaste con una versión anterior, lo que instalaron se queda en tu sistema; si ya no lo quieres, quítalo con tu gestor de paquetes. Como referencia, sus desinstaladores retiraban: Hyprland `nwg-look xwayland-satellite`; Niri esos dos y `dconf-editor`; Gaming `gamemode mangohud` (y `protonplus` en Arch), conservando Steam, Discord y Flatpak. Terminal Bonita guardaba tu `~/.zshrc` anterior como `~/.zshrc.backup_<marca de tiempo>`.

## 🧩 Un solo instalador, tres proyectos

GekkoApp es el punto de entrada de tres proyectos de The-Gekko. Cada uno se puede instalar por separado con su propio instalador; GekkoApp los orquesta desde una sola interfaz con desinstalación simétrica, pero no del mismo modo: Bauh y el propio GekkoApp se instalan desde un release verificado (manifiesto + SHA-256); Gekko ADB Studio se clona desde `main` y se instala con su `install.sh`, sin verificación.

> **Elige una sola vía por proyecto** (su instalador propio o GekkoApp); para cambiar de vía, desinstala primero con la misma con la que instalaste. Mezclarlas deja restos: GekkoApp retira el venv pipx `gekko-bauh` pero no los `.desktop`, iconos ni autostart que crea el `install.sh` de Bauh, y ese `install.sh uninstall` no conoce `org.thegekko.bauh.desktop` ni el estado de GekkoApp; para Gekko ADB ambas vías escriben las mismas rutas, así que la última que se ejecute gana.

| Proyecto | Qué es | Repositorio | Instalarlo por separado | Qué hace GekkoApp por él |
| :--- | :--- | :--- | :--- | :--- |
| **GekkoApp** (Control Center) | Esta app: instalador y actualizador de todo lo demás (Rust + Tauri v2). | [The-Gekko/GekkoApp](https://github.com/The-Gekko/GekkoApp) | `curl -fsSL https://raw.githubusercontent.com/The-Gekko/GekkoApp/main/scripts/install-release.sh \| bash`, o clonando este repositorio ([opción 2](#opción-2--clonando-el-repositorio-compila-con-cargo)). | Se auto-actualiza desde su último release (manifiesto + SHA-256, sin sudo) y reemplaza los symlinks de `~/.local/bin`. |
| **Gekko ADB Studio** | Suite GTK 3/4 de control ADB: scrcpy, shell, debloat, presets. | [The-Gekko/gekko-adb](https://github.com/The-Gekko/gekko-adb) | `curl -fsSL https://raw.githubusercontent.com/The-Gekko/gekko-adb/main/install.sh \| bash`, o clonando su repositorio ([README](https://github.com/The-Gekko/gekko-adb#readme)). | Instala con sudo los paquetes `git python python-gobject gtk3 gtk4 android-tools android-udev scrcpy glib2 xdg-utils xdg-user-dirs curl` (Arch) o `git python3 python-gobject libgtk-3 libgtk-4 android-tools scrcpy glib2 xdg-utils xdg-user-dirs curl` (Solus); clona `main` en `~/.cache/gekkoapp/gekko-adb`; ejecuta su `install.sh --no-deps --assume-yes`; en Arch recarga udev (`udevadm control --reload-rules && udevadm trigger`) si existe `51-android.rules`. |
| **bauh Gekko Edition** (Bauh Fork) | Fork de bauh: tienda gráfica para AUR, Chaotic AUR, Flatpak y eopkg. | [The-Gekko/The-Gekko-Bauh](https://github.com/The-Gekko/The-Gekko-Bauh) | `curl -fsSL https://raw.githubusercontent.com/The-Gekko/The-Gekko-Bauh/master/install.sh \| bash`, o clonando su repositorio ([README](https://github.com/The-Gekko/The-Gekko-Bauh#readme)). | Si existe el paquete `bauh` de pacman pide confirmación y lo desinstala (solo Arch); instala `python-pipx` (Arch) o `pipx` (Solus) si falta; resuelve `/releases/latest`, descarga manifiesto + `.tar.zst`, verifica tamaño y SHA-256; `pipx install --force` del árbol verificado; crea `org.thegekko.bauh.desktop` e icono hicolor 512. |

Notas honestas:

- **Gekko ADB Studio no tiene releases verificados (manifiesto + SHA-256)**: GekkoApp clona HEAD de `main` por HTTPS y no hay manifiesto ni SHA-256 que verificar. El diálogo de instalación lo dice tal cual. Por eso la campana no compara versiones, sino el commit que GekkoApp registró al instalar con el último de `main`: cualquier push a `main` aparece como actualización. Si Gekko ADB se instaló con su propio `install.sh`, GekkoApp no sabe qué commit tienes y la campana no dice nada (instalarlo o actualizarlo una vez desde GekkoApp lo resuelve).
- **Bauh** se instala como distribución `gekko-bauh` (desde `v0.10.8-gekko.1`), con los ejecutables `gekko-bauh`, `gekko-bauh-tray` y `gekko-bauh-cli` y **una sola entrada de menú** (`org.thegekko.bauh`). Hasta `v0.10.8-gekko.1` el release declaraba además la de la bandeja (`org.thegekko.bauh.tray`); al actualizar, GekkoApp retira esa entrada y el entorno pipx anterior para no dejar huérfanos. Las etiquetas con guion (`vX.Y.Z-gekko.N`) solo las acepta GekkoApp 1.2.0 o posterior.
- El repositorio antiguo `The-Gekko/Bauh-Fork-The-Gekko` solo se conserva como compatibilidad: los manifiestos ya publicados llevan ese nombre y GekkoApp los sigue aceptando.
- **Desinstalar desde GekkoApp** borra exactamente los archivos que creó en tu HOME; los paquetes del sistema que instaló con pacman/eopkg (dependencias de Gekko ADB, python-pipx/pipx) no se desinstalan. Gekko ADB: `~/.local/bin/gekko-adb`, `~/.local/share/applications/com.gekko.adb.desktop` (+ los legacy `GekkoADB.desktop` y `org.thegekko.gekko_adb.desktop`), `~/.local/share/metainfo/com.gekko.adb.metainfo.xml`, `~/.local/share/icons/hicolor/512x512/apps/gekko-adb.png`, `~/.local/share/gekko-adb` (app, `.env`, `app.bak.*`) y el clon `~/.cache/gekkoapp/gekko-adb`; conserva `~/.config/gekko-adb` y `~/.local/state/gekko-adb/logs`. Bauh: `pipx uninstall` del entorno registrado, los `.desktop` e iconos registrados (`org.thegekko.bauh`, y `org.thegekko.bauh.tray` si lo dejó un release anterior) y la copia del release; conserva la configuración del usuario (`~/.config/gekko-bauh`).

## 📦 Instalación y Desinstalación

Hay **exactamente dos formas** de instalar GekkoApp: **por `curl`** (opción 1, descarga el release verificado y no compila nada) o **clonando el repositorio** (opción 2, compila con `cargo`). Elige **una sola**: si cambias de vía, desinstala primero con el `--uninstall` de más abajo, que retira tanto los symlinks del release como los binarios copiados desde el código fuente.

### Requisitos

- Arch Linux, Garuda o Solus (o derivadas con `ID_LIKE=arch`/`solus`), `x86_64` y sesión con **systemd de usuario**.
- **Opción 1 (curl):** `curl`, `tar` (con zstd), `python3` y `getconf`. Además, **glibc** igual o superior a la mínima que declara el manifiesto del release (`platform.libc.minimum`): el instalador la compara con `getconf GNU_LIBC_VERSION` y aborta con un mensaje claro si el sistema es más antiguo. Esa mínima no se fija a mano: `scripts/build-release-bundle.sh` la deduce de los símbolos versionados de los binarios (`objdump -T` → mayor `GLIBC_x.y`) y la escribe en el manifiesto del release; sin `objdump` el empaquetado aborta en vez de adivinarla. Con la toolchain actual los binarios exigen **GLIBC 2.39**. Aviso: el release v1.1.0 declara 2.34 por un fallo del empaquetado ya corregido; si tu glibc es 2.34–2.38 el instalador no te avisará y el binario no arrancará. Desde la 1.2.0 cada release declara la mínima real.
- **Opción 2 (clonar):** `git` y `cargo`/`rustc` ([rustup](https://rustup.rs)), más las dependencias de compilación de Tauri v2 en Linux; en Arch son `base-devel`, `webkit2gtk-4.1`, `gtk3` y `libsoup3`. Aquí no hay comprobación de glibc: los binarios los compila tu propia máquina.

### Opción 1 — Por `curl` (release verificado, sin compilar)

```bash
curl -fsSL https://raw.githubusercontent.com/The-Gekko/GekkoApp/main/scripts/install-release.sh | bash
```

Opciones (`bash -s -- <opción>`): `--version vX.Y.Z` (una versión concreta), `--prefix <dir>` (por defecto `~/.local`), `--no-launch` (no abrir el Control Center al terminar), `--uninstall`, `--help`.

Qué crea:

- `~/.local/lib/kitotsu/gekkoapp/<versión>/` — el release extraído y verificado (una carpeta por versión; si la versión ya está instalada no se vuelve a descargar, solo se reactiva).
- `~/.local/bin/gekkoapp-gui` y `~/.local/bin/gekkoapp` — symlinks a esa versión (Control Center y CLI).
- `~/.local/share/applications/org.thegekko.gekkoapp.desktop` — entrada de menú.
- `~/.local/share/icons/hicolor/512x512/apps/org.thegekko.gekkoapp.png` y `.../symbolic/apps/org.thegekko.gekkoapp-symbolic.svg` — iconos.

Al terminar se abre el Control Center: **desde ahí instalas, actualizas o desinstalas cualquier componente**.

### Opción 2 — Clonando el repositorio (compila con `cargo`)

```bash
git clone https://github.com/The-Gekko/GekkoApp.git
cd GekkoApp
./scripts/install.sh            # cargo build --locked --release (+ --features gui) e instala en ~/.local
```

`scripts/install.sh` es idempotente y no usa `sudo`. Qué crea:

- `~/.local/bin/gekkoapp` y `~/.local/bin/gekkoapp-gui` — copias de los binarios recién compilados (CLI y Control Center). Si en esas rutas había symlinks del motor de releases (opción 1), los retira avisando.
- `~/.local/share/applications/org.thegekko.gekkoapp.desktop` — entrada de menú (mismo nombre que la de la opción 1, para no duplicarla).
- `~/.local/share/icons/hicolor/512x512/apps/org.thegekko.gekkoapp.png` y `.../symbolic/apps/org.thegekko.gekkoapp-symbolic.svg` — iconos.

Variantes útiles dentro de esta misma vía:

```bash
GEKKOAPP_SKIP_BUILD=1 ./scripts/install.sh   # reinstala los binarios ya compilados (no requiere cargo)
GEKKOAPP_PREFIX=~/opt/gekko ./scripts/install.sh  # instala en otro prefijo (por defecto ~/.local)
./GekkoApp.sh                   # Control Center desde el clon (compila con cargo la primera vez)
./GekkoApp.sh --cli             # menú en terminal, con submenú de desinstalación [d]
gekkoapp --help                 # ayuda del CLI; gekkoapp --version imprime la versión
```

El CLI acepta `--help`/`-h` y `--version`/`-V`, sale con código 2 ante una opción desconocida y termina limpiamente si la entrada está cerrada (`gekkoapp </dev/null`).

Para desinstalar lo que dejó esta vía se usa el mismo `--uninstall` de más abajo: reconoce y retira también los binarios copiados por `scripts/install.sh`.

### Desinstalar GekkoApp por completo

```bash
curl -fsSL https://raw.githubusercontent.com/The-Gekko/GekkoApp/main/scripts/install-release.sh | bash -s -- --uninstall
```

Retira los dos symlinks (o los binarios que dejó `scripts/install.sh`, si son de GekkoApp), la entrada de menú, los iconos, todas las versiones de `~/.local/lib/kitotsu/gekkoapp` y los directorios que queden vacíos; también retira la entrada `gekkoapp` del estado del motor (`~/.local/state/gekkoapp/installations-v1.json`, sin tocar los demás módulos) y el artefacto `gekkoapp-*.tar.zst` que el auto-update dejó en `~/.cache/gekkoapp/artifacts`. No toca los componentes instalados desde el Control Center: desinstálalos antes desde la GUI o el CLI si no los quieres.

## 🔧 Verificación y desarrollo

El crate está en `Gekko APP/gekkoapp-rs` (no hay `Cargo.toml` en la raíz). Puerta de calidad, en orden:

```bash
cd "Gekko APP/gekkoapp-rs"
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo build --locked --release
cargo build --locked --release --features gui --bin gekkoapp-gui
cargo test --locked --all-features -- --ignored resolves_published gekko_adb_update   # requiere red: consulta GitHub de verdad
GEKKOAPP_BAUH_DIST=<dist de Bauh> cargo test --locked -- --ignored consumes_a_generated   # release de Bauh generado con scripts/build-bauh-release.sh
```

### Publicar una versión

`.github/workflows/ci.yml` pasa esa misma puerta (más `shellcheck` y la comparación de los lanzadores) en cada push a `main`. `.github/workflows/release.yml` compila y publica el release al empujar una etiqueta `vX.Y.Z`, en Ubuntu 24.04 (glibc 2.39): no hace falta Rust en tu máquina. Pasos:

1. Sube la versión en `Gekko APP/gekkoapp-rs/Cargo.toml`, `Cargo.lock` (entrada `gekkoapp`), `tauri.conf.json` y la línea «Versión» de este README.
2. `git push` y espera a que `ci` salga en verde.
3. `git tag -a vX.Y.Z -m "GekkoApp X.Y.Z" && git push origin vX.Y.Z`.

El workflow comprueba que la etiqueta coincide con `Cargo.toml` y `tauri.conf.json`, pasa los tests, compila, empaqueta con `scripts/build-release-bundle.sh`, verifica el manifiesto contra el artefacto y publica `gekkoapp-X.Y.Z.tar.zst`, `gekkoapp-X.Y.Z.sha256` y `gekkoapp-x86_64-unknown-linux-gnu.manifest.json`. Para repetir la publicación de una etiqueta existente: *Actions → release → Run workflow* con la etiqueta.

Publicar Bauh o cambiar Gekko ADB **no requiere tocar GekkoApp**: la campana los detecta sola (ver arriba).

Scripts de release (desde la raíz del repositorio; `release.yml` usa el primero):

- `scripts/build-release-bundle.sh` — empaqueta `releases/dist/gekkoapp-<versión>.tar.zst`, `.sha256` y el manifiesto (`kitotsu.release-artifact` 1.0, `binary_extract`). La glibc mínima la deduce de los binarios con `objdump -T`; si falta `objdump` aborta en vez de adivinarla.
- `scripts/build-bauh-release.sh` — empaqueta un release de Bauh para el motor de GekkoApp (`python_pipx`): `product.version = X.Y.Z+gekko.N`, `release.tag = vX.Y.Z-gekko.N` (etiqueta git con guion) y artefacto `bauh-fork-the-gekko-X.Y.Z.gekko.N.tar.zst`. El mismo script vive en el fork como `tools/build-gekkoapp-release.sh`, que ejecuta su `release.yml` en cada etiqueta `v*`.
- `shellcheck` sobre `scripts/*.sh`, `GekkoApp.sh` y `Gekko APP/GekkoApp.sh` (los dos lanzadores son byte-idénticos: `cmp -s`).

## 👥 Colaboradores

| Colaborador | Rol |
| :---------- | :--- |
| **The-Gekko** | Arquitectura en Rust y lógica de sistema. |
| **𝓲𝓑𝓵𝓾𝓮𝓜𝓸𝓸𝓷** | Especialista en shell: creó Terminal Bonita (Zsh + plugins), incluida hasta la versión 1.2.0. |
| **KitotsuMolina** | Ecosistema Kito (KiUI, Kitsune, Kitowall, Kilivepaper, KiSDDM). |

## ☕ Apoya el proyecto

¿Te ha ahorrado tiempo? [Invítame un café](https://gravatar.com/thegekko5) para seguir mejorando GekkoApp.

## 📄 Licencia

**zlib/libpng** (SPDX `Zlib`), (c) 2026 The-Gekko — véase [`LICENSE`](LICENSE) · Desarrollado con ❤️ para la comunidad de Linux por **The-Gekko** y colaboradores.
