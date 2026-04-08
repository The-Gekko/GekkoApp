# 🚀 GekkoApp: Linux Personalizer & Gaming Setup

**GekkoApp** ha evolucionado. Lo que empezó como un script de automatización ahora es una herramienta robusta desarrollada en **Rust**, diseñada para ofrecer la máxima velocidad, seguridad y una experiencia minimalista en la terminal.

## ✨ Características Principales

Esta herramienta automatiza la configuración de tu entorno, evitando tareas tediosas y errores manuales:

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

### 1\. Opción Rápida (Recomendado)

Descarga el binario ya compilado desde nuestra sección de **[Releases]**.

```bash
# Dale permisos de ejecución
chmod +x GekkoApp

# Ejecútalo
./GekkoApp
```

### 2\. Compilación desde Fuente

Si prefieres compilarlo tú mismo, asegúrate de tener el entorno de Rust instalado:

```bash
git clone https://github.com/The-Gekko/GekkoApp.git
cd GekkoApp
cargo build --release
./target/release/gekkoapp
```

> [\!IMPORTANT]
> Se recomienda ejecutar esta herramienta en una instalación limpia o realizar un backup de tu carpeta `~/.config` antes de empezar.

---

## 🛠️ Requisitos

- **Arch Linux** o derivados.
- Conexión a internet estable.
- Privilegios de **sudo**.
  ![image.png](image.png)

---

## 📸 Implementaciones

| Terminal (Kitty + Zsh) | Gaming Setup |
| :--------------------- | :----------- |

\*Configuración de Zsh y plugins por **𝓲𝓑𝓵𝓾𝓮𝓜𝓸𝓸𝓷\***.

---

## 📄 Licencia

Este proyecto está bajo la Licencia **MIT**. Consulta el archivo `LICENSE` para más detalles.

Desarrollado con ❤️ para la comunidad de Linux por **The-Gekko** y colaboradores.
