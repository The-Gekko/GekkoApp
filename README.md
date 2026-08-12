<p align="center">
  <img src="Gekko%20APP.png" alt="GekkoApp" width="420"/>
  <br/>
  <em>Imagen hecha con IA · Gemini en su modelo Nano Banana</em>
</p>

# 🚀 GekkoApp: Linux Personalizer & Gaming Setup

**GekkoApp** ha evolucionado. Lo que empezó como un script de automatización ahora es una herramienta robusta desarrollada en **Rust**, para máxima velocidad, seguridad y una experiencia minimalista. **Todo se controla desde el Control Center de escritorio (Tauri v2):** desde ahí instalas, actualizas y recibes las nuevas actualizaciones con un clic.

## ✨ Características Principales

Esta herramienta automatiza la configuración de tu entorno, evitando tareas tediosas y errores manuales:

### 🖥️ Control Center (Interfaz de Escritorio)

- Aplicación de escritorio **Tauri v2** (`gekkoapp-gui`) que controla **todo** el post-install: Entorno Kito, Tienda Bauh Fork, Gekko ADB Studio, GekkoApp (auto-update), Terminal Bonita (ZSH + Starship), presets Hyprland y Niri, Gaming Setup (NVIDIA/Intel/AMD) y Chaotic AUR.
- Progreso y logs en vivo; verifica cada release firmado (SHA-256) antes de tocar el sistema.
- **Auto-update de GekkoApp:** la propia aplicación se actualiza a sí misma desde un release firmado de GitHub (`binary_extract`, mismo motor que Bauh) sin necesidad de sudo.
- **Campana de actualizaciones:** al abrir el Control Center consulta la última versión de cada componente con releases (Kito, Bauh Fork, GekkoApp) y avisa de las actualizaciones disponibles.
- Los componentes del catálogo (Kito, Bauh Fork, Gekko ADB Studio, GekkoApp) muestran su versión instalada y botones **Instalar / Actualizar**.
- **Tema adaptativo:** detecta automáticamente la paleta de **matugen** y recolorea el propio Control Center, siguiendo en vivo los cambios de wallpaper.

### 🦊 Entorno Kito (Auto-Update)

- Instalación y **actualización automática** de KiUI, Kitsune Compositor y los módulos Kitowall, Kilivepaper y KiSDDM desde **GitHub Releases** firmados.
- Detección de distribución, arquitectura, sesión y escritorio; resuelve y valida los releases antes de modificar el sistema.

### 🛍️ Tienda Bauh Fork (Auto-Update)

- Instala o actualiza **Bauh Fork (The-Gekko)** desde un release firmado de GitHub: verificación del manifiesto SHA-256 e instalación aislada con `pipx`. Nunca clona el repositorio ni ejecuta scripts sin verificar.

### 🔄 GekkoApp (Auto-Update)

- GekkoApp se actualiza a **sí misma** con el mismo motor de releases firmados: resuelve la última versión publicada de `The-Gekko/GekkoApp`, verifica el manifiesto (contrato `kitotsu.release-artifact` 1.0 + SHA-256) y activa los binarios con symlinks en `~/.local/bin`, regenerando la entrada de menú y los iconos. No requiere sudo.
- La campana 🔔 del Control Center consulta las actualizaciones disponibles al abrir (y tras cada instalación) para Kito, Bauh Fork y el propio GekkoApp.

### 📱 Gekko ADB Studio (Actualización desde fuente)

- Instala o actualiza **Gekko ADB Studio** (suite GTK de control ADB: scrcpy, shell, debloat y presets) desde el código fuente del repo `The-Gekko/gekko-adb`: instala las dependencias de sistema (GTK3/4, `android-tools`, `scrcpy`) y ejecuta su instalador oficial (`install.sh --no-deps`), registrando launcher, icono y entrada de menú.

### 🎨 Tema adaptativo (Matugen · Material You)

- **GekkoApp no instala nada**: si el sistema ya usa matugen (QuickShell/HyDE en Garuda regeneran `~/.cache/matugen/colors-gtk.css` al cambiar el wallpaper), el Control Center detecta esa paleta al abrirse y la aplica a su propia interfaz.
- GTK3 y GTK4 ya importan ese mismo CSS, de modo que **GekkoApp sigue los mismos colores que las demás aplicaciones**, igual que el shell/dock de HyDE.
- Un watcher escucha el archivo de paleta: si cambias el wallpaper con la app abierta, el Control Center se **recolorea en vivo** (evento `theme://changed`).
- Si no hay paleta de matugen, GekkoApp mantiene su tema oscuro por defecto.

### 💻 Terminal & Shell (Optimizado por 𝓲𝓑𝓵𝓾𝓮𝓜𝓸𝓸𝓷)

- **Kitty Terminal:** Configuración avanzada con soporte para ligaduras y temas.
- **Zsh + Plugins:** Instalación automática y tuning de:
  - `zsh-syntax-highlighting` (resaltado en tiempo real).
  - `zsh-autosuggestions` (inteligencia basada en historial).
  - Plugins exclusivos para productividad máxima.

### 🛠️ Ecosistema Wayland (Hyprland & Niri)

- Instalación de herramientas esenciales para **Hyprland** y **Niri**.
- Setup de barras de estado, notificaciones y lanzadores.

### 📦 Repositorios & Paquetes

- **Chaotic-AUR:** Integración rápida para obtener kernels optimizados y binarios pre-compilados (ahorra horas de compilación).

### 🎮 Gaming & Drivers (Power by The-Gekko)

- **Detección de Hardware:** Configuración específica para **NVIDIA**, **AMD** o **Intel**.
- **Stack Gaming:** Gamemode, Wine-staging, Lutris y dependencias de Vulkan.

---

## 👥 Equipo y Colaboradores

| Colaborador   | Rol y Responsabilidades                                                    |
| :------------ | :------------------------------------------------------------------------- |
| **The-Gekko** | Arquitectura en Rust, Lógica de Sistema y Optimización Gaming.             |
| **𝓲𝓑𝓵𝓾𝓮𝓜𝓸𝓸𝓷** | Especialista en Shell. Diseño de experiencia Zsh e integración de plugins. |
| **KitotsuMolina** | Desarrollador del ecosistema Kito (KiUI, Kitsune, Kitowall, Kilivepaper, KiSDDM). |

---

## 📦 Instalación

> [!IMPORTANT]
> **Todo se controla desde el Control Center (GUI).** El lanzador `GekkoApp.sh` abre
> el Control Center por defecto; la primera vez compila los binarios Rust con cargo.
> Ya no existe fallback en bash.

### 🚀 Instalación recomendada (un comando)

```bash
./scripts/install.sh
```

Compila el CLI y el Control Center (`--locked`), los instala en `~/.local/bin`,
registra la aplicación en el menú con su icono y refresca las caches del escritorio.
Es idempotente (re-ejecutable sin efectos) y usa `GEKKOAPP_PREFIX=/ruta` para otro
prefijo de instalación.

### ▶️ Arranque del Control Center

```bash
./GekkoApp.sh             # abre el Control Center (GUI)
./GekkoApp.sh --cli       # menú en terminal (opcional: SSH / power users)
```

Tras `./scripts/install.sh` también puedes lanzarlo directamente con
`~/.local/bin/gekkoapp-gui` (si `~/.local/bin` está en tu PATH) o desde el menú
de aplicaciones como **«GekkoApp Control Center»** (rofi/waybar/el launcher que uses).

### 🔧 Solo para probar sin instalar

```bash
cd "Gekko APP/gekkoapp-rs"
cargo run --release --features gui --bin gekkoapp-gui
```

> [!IMPORTANT]
> La GUI requiere las bibliotecas de desarrollo de **WebKitGTK 4.1**, GTK3 y libsoup3
> (en Arch/Garuda: `webkit2gtk-4.1`, `gtk3`, `libsoup3`). La CLI se compila sin ellas.

### Interfaz de terminal (CLI)

El Control Center es la interfaz principal. La CLI `gekkoapp` sigue disponible
(la instala `scripts/install.sh`) con el mismo motor Rust, útil por SSH/TTY:

```bash
./GekkoApp.sh --cli
# o directamente:
cargo build --release        # desde "Gekko APP/gekkoapp-rs"
./target/release/gekkoapp
```

> [!IMPORTANT]
> Se recomienda ejecutar esta herramienta en una instalación limpia o realizar un backup de tu carpeta `~/.config` antes de empezar.

### Empaquetado de release

`scripts/build-release-bundle.sh` genera en `releases/dist/` un tarball con los
binarios, launchers, entrada `.desktop`, icono y README, más su checksum SHA-256 y
un manifiesto de contrato (`kitotsu.release-artifact` 1.0), listo para publicar en
GitHub Releases. `scripts/build-bauh-release.sh` hace lo mismo para el Bauh Fork.

---

### Entorno Kito

El Control Center inicia el instalador del entorno Kito con un clic (en la CLI,
opción `k`). GekkoApp detecta la distribución, arquitectura, sesión gráfica,
escritorio y gestor de servicios, permite corregir falsos positivos y resuelve
los releases necesarios antes de modificar el sistema.

La primera matriz soportada es Arch Linux `x86_64`, Wayland, Hyprland y servicios
de usuario de systemd. KiUI y Kitsune Compositor son componentes obligatorios;
Kitowall, Kilivepaper y KiSDDM se seleccionan como módulos independientes.
Kitsune se muestra como próximamente y permanece deshabilitado hasta publicar su
release.

El diseño y estado de implementación se documentan en
[`docs/entorno-kito.md`](docs/entorno-kito.md).

### Tienda Bauh Fork

El Control Center instala o actualiza Bauh Fork desde su release firmado en
GitHub con un clic (en la CLI, opción `8`): manifiesto SHA-256 + `pipx install
--force`. Si tienes el `bauh` oficial de pacman instalado, GekkoApp te pedirá
confirmación para desinstalarlo y evitar conflictos.

### Actualizar GekkoApp

El botón **Actualizar GekkoApp** del Control Center (y la campana 🔔, que avisa
cuando hay una versión nueva) actualiza la propia GekkoApp desde su release
firmado, igual que el propio instalador de cada componente. En la CLI es la
opción `u`.

## 🛠️ Requisitos

- **Arch Linux** o derivados.
- Conexión a internet estable.
- Privilegios de **sudo**.
- (Solo GUI) WebKitGTK 4.1 y dependencias de Tauri.

---

## 📸 Implementaciones

| Terminal (Kitty + Zsh) | Gaming Setup |
| :--------------------- | :----------- |

\*Configuración de Zsh y plugins por **𝓲𝓑𝓵𝓾𝓮𝓜𝓸𝓸𝓷\***.

---

## ☕ Apoya el Proyecto

Si **GekkoApp** te ha ahorrado tiempo y te ha ayudado a dejar tu sistema a punto, considera invitarme un café para apoyar el desarrollo continuo y el mantenimiento de la herramienta. ¡Toda ayuda es bienvenida para seguir mejorando!

- **[Invítame un café aquí]** _(https://gravatar.com/thegekko5)_

---

## 📄 Licencia

Este proyecto está bajo la Licencia **MIT**. Consulta el archivo `LICENSE` para más detalles.

Desarrollado con ❤️ para la comunidad de Linux por **The-Gekko** y colaboradores.
