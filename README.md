<p align="center">
  <img src="Gekko%20APP.png" alt="GekkoApp" width="320"/>
  <br/>
  <em>Imagen hecha con IA · Gemini en su modelo Nano Banana</em>
</p>

# 🚀 GekkoApp: Linux Personalizer & Gaming Setup

**GekkoApp** es un **Control Center de escritorio** (Rust + Tauri v2) para Arch Linux, Garuda y Solus. Con un clic instalas, actualizas, desinstalas y mantienes todo tu entorno sin tocar la terminal.

## ✨ Lo que trae

- 🖥️ **Control Center (GUI)** — desde aquí lo controlas todo: instalar, actualizar, desinstalar componentes y ver novedades con la campana 🔔.
- 🐧 **Soporte Multi-Distro (Arch, Garuda y Solus)** — detecta `pacman` y `eopkg` automáticamente y adapta los paquetes, plugins y vistas.
- 🗑️ **Desinstalación limpia e idempotente** — desinstala Kito, Bauh, Gekko ADB, Terminal Bonita, Presets y Gaming desde la GUI o el CLI sin dejar residuos.
- 🦊 **Entorno Kito** — KiUI, Kitsune Compositor y módulos (Kitowall, Kilivepaper, KiSDDM) desde releases firmados.
- 🛍️ **Tienda Bauh Fork** — instalación aislada con `pipx`, con verificación SHA-256.
- 📱 **Gekko ADB Studio** — suite GTK de control ADB (scrcpy, shell, debloat y presets).
- 🔄 **Auto-update de GekkoApp** — la app se actualiza a sí misma desde un release firmado, sin sudo.
- 💻 **Terminal Bonita** — ZSH + Starship + plugins por **𝓲𝓑𝓵𝓾𝓮𝓜𝓸𝓸𝓷**.
- 🪟 **Presets Hyprland y Niri** — herramientas y dependencias listas (Arch Linux).
- 🎮 **Gaming Setup** — NVIDIA, Intel o AMD (optimizado para Arch y Solus).
- 📦 **Chaotic AUR** — repositorios optimizados en un clic (Arch Linux).
- 🎨 **Tema adaptativo** — sigue la paleta de matugen en vivo (Material You).

Cada release se verifica (HTTPS + SHA-256) antes de tocar tu sistema. **Nada de `curl | sh` de terceros.**

## 📦 Instalación y Desinstalación

Instala el Control Center en un comando (sin compilar):

```bash
curl -fsSL https://raw.githubusercontent.com/The-Gekko/GekkoApp/main/scripts/install-release.sh | bash
```

Al terminar se abre el Control Center: **desde ahí instalas, actualizas o desinstalas cualquier componente**.

Para desinstalar GekkoApp por completo:
```bash
curl -fsSL https://raw.githubusercontent.com/The-Gekko/GekkoApp/main/scripts/install-release.sh | bash -s -- --uninstall
```

> Para desarrolladores (desde el código fuente):
> ```bash
> ./scripts/install.sh
> ./GekkoApp.sh --cli   # o el menú en terminal con opción de desinstalación [d]
> ```


## 👥 Colaboradores

| Colaborador | Rol |
| :---------- | :--- |
| **The-Gekko** | Arquitectura en Rust, lógica de sistema y optimización gaming. |
| **𝓲𝓑𝓵𝓾𝓮𝓜𝓸𝓸𝓷** | Especialista en shell: experiencia Zsh e integración de plugins. |
| **KitotsuMolina** | Ecosistema Kito (KiUI, Kitsune, Kitowall, Kilivepaper, KiSDDM). |

## ☕ Apoya el proyecto

¿Te ha ahorrado tiempo? [Invítame un café](https://gravatar.com/thegekko5) para seguir mejorando GekkoApp.

## 📄 Licencia

MIT · Desarrollado con ❤️ para la comunidad de Linux por **The-Gekko** y colaboradores.
