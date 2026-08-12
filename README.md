# 🚀 GekkoApp: Linux Personalizer & Gaming Setup

**GekkoApp** ha evolucionado. Lo que empezó como un script de automatización ahora es una herramienta robusta desarrollada en **Rust**, diseñada para ofrecer la máxima velocidad, seguridad y una experiencia minimalista en la terminal, con un **Control Center** de escritorio (Tauri v2) para instalar y actualizar tu entorno con un clic.

## ✨ Características Principales

Esta herramienta automatiza la configuración de tu entorno, evitando tareas tediosas y errores manuales:

### 🖥️ Control Center (Interfaz de Escritorio)

- Aplicación de escritorio **Tauri v2** (`gekkoapp-gui`) con catálogo de componentes.
- Botones **Instalar / Actualizar** para el Entorno Kito y la Tienda Bauh Fork.
- Progreso y logs en vivo; verifica cada release firmado (SHA-256) antes de tocar el sistema.

### 🦊 Entorno Kito (Auto-Update)

- Instalación y **actualización automática** de KiUI, Kitsune Compositor y los módulos Kitowall, Kilivepaper y KiSDDM desde **GitHub Releases** firmados.
- Detección de distribución, arquitectura, sesión y escritorio; resuelve y valida los releases antes de modificar el sistema.

### 🛍️ Tienda Bauh Fork (Auto-Update)

- Instala o actualiza **Bauh Fork (The-Gekko)** desde un release firmado de GitHub: verificación del manifiesto SHA-256 e instalación aislada con `pipx`. Nunca clona el repositorio ni ejecuta scripts sin verificar.

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

---

## 📦 Instalación

El proyecto **no es un crate en la raíz**: el paquete Rust está en `Gekko APP/gekkoapp-rs`. Ejecuta los comandos desde esa carpeta.

### Interfaz de escritorio (Control Center)

```bash
cd "Gekko APP/gekkoapp-rs"
cargo build --release --features gui --bin gekkoapp-gui
./target/release/gekkoapp-gui
```

> [!IMPORTANT]
> La interfaz GUI requiere las bibliotecas de desarrollo de **WebKitGTK 4.1**, GTK3 y libsoup3 (en Arch/Garuda: `webkit2gtk-4.1`, `gtk3`, `libsoup3`). La CLI se compila sin estas dependencias.

### Interfaz de terminal (CLI)

```bash
cd "Gekko APP/gekkoapp-rs"
cargo build --release
./target/release/gekkoapp
```

> [!IMPORTANT]
> Se recomienda ejecutar esta herramienta en una instalación limpia o realizar un backup de tu carpeta `~/.config` antes de empezar.

### Launchers

Los launchers `GekkoApp.sh` y `Gekko APP/GekkoApp.sh` (byte-idénticos) priorizan los
binarios Rust: si `target/release/gekkoapp` existe lo ejecutan; si no, compilan con
`cargo run --release`. El flag `--gui` lanza el Control Center de escritorio:

```bash
./GekkoApp.sh            # menú en terminal (o binario CLI si ya compilado)
./GekkoApp.sh --gui      # Control Center (Tauri v2)
```

### Instalación con un comando

`scripts/install.sh` compila ambos binarios (`--locked`), los instala en
`~/.local/bin` y registra el Control Center en el menú de aplicaciones (icono y
entrada `.desktop`). Es idempotente; usa `GEKKOAPP_PREFIX=/ruta` para otro prefijo.

```bash
./scripts/install.sh
```

### Empaquetado de release

`scripts/build-release-bundle.sh` genera en `releases/dist/` un tarball con los
binarios, launchers, entrada `.desktop`, icono y README, más su checksum SHA-256 y
un manifiesto de contrato (`kitotsu.release-artifact` 1.0), listo para publicar en
GitHub Releases. `scripts/build-bauh-release.sh` hace lo mismo para el Bauh Fork.

---

### Entorno Kito

La opción `K` inicia el instalador del entorno Kito. GekkoApp detecta la
distribución, arquitectura, sesión gráfica, escritorio y gestor de servicios,
permite corregir falsos positivos y resuelve los releases necesarios antes de
modificar el sistema.

La primera matriz soportada es Arch Linux `x86_64`, Wayland, Hyprland y servicios
de usuario de systemd. KiUI y Kitsune Compositor son componentes obligatorios;
Kitowall, Kilivepaper y KiSDDM se seleccionan como módulos independientes.
Kitsune se muestra como próximamente y permanece deshabilitado hasta publicar su
release.

El diseño y estado de implementación se documentan en
[`docs/entorno-kito.md`](docs/entorno-kito.md).

### Tienda Bauh Fork

La opción `8` instala o actualiza Bauh Fork desde su release firmado en GitHub
(manifiesto SHA-256 + `pipx install --force`). Si tienes el `bauh` oficial de
pacman instalado, GekkoApp te pedirá confirmación para desinstalarlo y evitar
conflictos.

## 🛠️ Requisitos

- **Arch Linux** o derivados.
- Conexión a internet estable.
- Privilegios de **sudo**.
- (Solo GUI) WebKitGTK 4.1 y dependencias de Tauri.

---

## 📸 Proyecto

![image.png](image.png)

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
