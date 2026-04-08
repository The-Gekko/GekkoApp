# 🚀 Linux Personalizer & Gaming Setup

Una herramienta integral diseñada para automatizar la configuración de tu entorno Linux, optimizar tu terminal y dejar tu sistema listo para el gaming y la productividad.

## ✨ Características Principales

Este script facilita la transición a un entorno personalizado, encargándose de las tareas tediosas por ti:

### 💻 Terminal & Shell

- **Kitty Terminal:** Configuración optimizada con soporte para ligaduras y temas.
- **Zsh + Plugins:** Instalación automática de `zsh` junto con:
  - `zsh-syntax-highlighting` (resaltado de comandos).
  - `zsh-autosuggestions` (sugerencias basadas en historial).
  - Integraciones personalizadas y optimización de rendimiento.

### 🛠️ Ecosistema Wayland (Hyprland & Niri)

- Instalación de herramientas esenciales para **Hyprland** y **Niri**.
- Configuración de barras de estado, notificaciones y selectores de aplicaciones.

### 📦 Repositorios & Paquetes

- **Chaotic-AUR:** Configuración rápida del repositorio para obtener binarios pre-compilados y kernels optimizados sin esperar horas de compilación.

### 🎮 Gaming & Drivers

Optimización completa según tu hardware para que solo tengas que abrir Steam y jugar:

- **NVIDIA:** Instalación de drivers propietarios y parches de Wayland.
- **AMD/Intel:** Configuración de Mesa y drivers Vulkan.
- **Herramientas:** Instalación de Gamemode, Wine-staging, Lutris y dependencias necesarias.

---

## 👥 Equipo y Colaboradores

Este proyecto es posible gracias al trabajo conjunto de:

| Colaborador   | Rol y Responsabilidades                                                                             |
| :------------ | :-------------------------------------------------------------------------------------------------- |
| **The-Gekko** | Desarrollador Principal, Lógica de Sistema en Rust y Herramientas Gaming.                           |
| **𝓲𝓑𝓵𝓾𝓮𝓜𝓸𝓸𝓷** | Especialista en Shell. Encargado de la modificación de **Zsh**, integraciones y gestión de plugins. |

---

## 🚀 Instalación (Compilación desde fuente)

Al estar desarrollado en **Rust**, garantizamos velocidad y seguridad. Para compilar y ejecutar la herramienta:

```bash
# Clonar el repositorio
git clone https://github.com/The-Gekko/GekkoApp.git
cd GekkoApp

# Compilar la versión de producción
cargo build --release

# Ejecutar el binario
./target/release/gekkoapp
```

> [\!IMPORTANT]
> Se recomienda ejecutar esta herramienta en una instalación limpia o tener un backup de tus archivos de configuración actuales (`.config`).

---

## 🛠️ Requisitos

- Una distribución basada en **Arch Linux** (recomendado).
- **Rust & Cargo** instalados para la compilación.
- Conexión a internet estable y permisos de **sudo**.

---

## 📸 Implementaciones

| Terminal (Kitty + Zsh) | Gaming Setup |
| :--------------------- | :----------- |

| _(Zsh by 𝓲𝓑𝓵𝓾𝓮𝓜𝓸𝓸𝓷)_

![image.png](image.png)

---

Desarrollado con ❤️ por **The-Gekko** y colaboradores.
