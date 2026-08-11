#!/usr/bin/env bash

# ==============================================================================
# GekkoApp - Arch Linux Post-Installation & Configuration Tool
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_APP_DIR="$SCRIPT_DIR/Gekko APP/gekkoapp-rs"
if [ ! -d "$RUST_APP_DIR" ]; then
    RUST_APP_DIR="$SCRIPT_DIR/gekkoapp-rs"
fi
RELEASE_BIN="$RUST_APP_DIR/target/release/gekkoapp"

# Priorizar el binario en Rust auditado si está compilado
if [ -x "$RELEASE_BIN" ]; then
    exec "$RELEASE_BIN" "$@"
fi

# Si cargo está disponible, compilar y ejecutar la versión oficial en Rust
if command -v cargo >/dev/null 2>&1 && [ -d "$RUST_APP_DIR" ]; then
    echo "Iniciando GekkoApp (Rust)..."
    cd "$RUST_APP_DIR" && exec cargo run --release -- "$@"
fi

# ==================================================== #
#                  FALLBACK SHELL SCRIPT               #
# ==================================================== #

set -e

C_DEF='\033[0m'
C_CYAN='\033[1;36m'
C_GREEN='\033[1;32m'
C_YELLOW='\033[1;33m'
C_RED='\033[1;31m'
C_BLUE='\033[1;34m'
C_PURPLE='\033[1;35m'
C_WHITE='\033[1;37m'

print_title() {
    clear
    echo -e "${C_CYAN}"
    echo "╭──────────────────────────────────────────────────────────────╮"
    echo "│                                                              │"
    echo "│         🐉 THE-GEKKO LINUX POST-INSTALL & CONFIG 🐉          │"
    echo "│                                                              │"
    echo "╰──────────────────────────────────────────────────────────────╯"
    echo -e "${C_DEF}"
}

print_header() {
    echo -e "\n${C_PURPLE}═══════════════════════════════════════════════════════════════"
    echo -e "🔮 $1"
    echo -e "═══════════════════════════════════════════════════════════════${C_DEF}\n"
}

print_success() { echo -e "${C_GREEN}✅ $1${C_DEF}"; }
print_warning() { echo -e "${C_YELLOW}⚠️ $1${C_DEF}"; }
print_info() { echo -e "${C_BLUE}ℹ️ $1${C_DEF}"; }

instalar_paquetes() {
    local paquetes=("$@")
    local faltantes=()

    for pkg in "${paquetes[@]}"; do
        if ! pacman -Qq "$pkg" >/dev/null 2>&1; then
            faltantes+=("$pkg")
        fi
    done

    if [ ${#faltantes[@]} -eq 0 ]; then
        print_success "Todos los paquetes necesarios ya están instalados."
    else
        print_info "Instalando paquetes faltantes: ${faltantes[*]}"
        sudo pacman -S --needed --noconfirm "${faltantes[@]}"
    fi
}

install_zsh_starship() {
    print_header "INSTALANDO TERMINAL BONITA (ZSH + STARSHIP)"
    instalar_paquetes kitty git zsh nano curl eza fastfetch fzf ttf-jetbrains-mono-nerd starship zsh-autosuggestions zsh-syntax-highlighting zsh-history-substring-search zsh-completions zoxide
    print_success "Terminal ZSH y plugins instalados."
    sleep 2
}

install_hyprland() {
    print_header "INSTALANDO PRESET HYPRLAND"
    instalar_paquetes gedit electron gnome-keyring polkit-gnome xdg-desktop-portal-gtk xwayland-satellite nwg-look blueman ibus unzip colord bolt flatpak gnome-disk-utility gvfs-mtp gvfs-gphoto2 libmtp nautilus fuse2 ddcutil i2c-dev
    print_success "Evaluación de dependencias de Hyprland finalizada."
    sleep 2
}

install_niri() {
    print_header "INSTALANDO PRESET NIRI"
    instalar_paquetes gedit xdg-desktop-portal-gnome gnome-keyring polkit-gnome xdg-desktop-portal-gtk electron xwayland-satellite gnome-disk-utility qt5-wayland qt6-wayland nwg-look flatpak blueman gvfs-mtp gvfs-gphoto2 libmtp ibus unzip colord bolt fuse2 ddcutil i2c-dev playerctl gpsd dconf-editor
    print_success "Evaluación de dependencias de Niri finalizada."
    sleep 2
}

install_chaotic_aur() {
    print_header "AGREGANDO REPOSITORIOS CHAOTIC AUR"
    if grep -q "\[chaotic-aur\]" /etc/pacman.conf; then
        print_success "Chaotic AUR ya se encuentra configurado en pacman.conf. Saltando..."
    else
        print_info "Obteniendo y verificando llaves públicas..."
        sudo pacman-key --recv-key 3056513887B78AEB --keyserver keyserver.ubuntu.com
        sudo pacman-key --lsign-key 3056513887B78AEB

        print_info "Instalando paquete keyring y mirrorlist..."
        sudo pacman -U --noconfirm 'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst'
        sudo pacman -U --noconfirm 'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst'

        print_info "Editando /etc/pacman.conf en automático..."
        echo -e "\n[chaotic-aur]\nInclude = /etc/pacman.d/chaotic-mirrorlist" | sudo tee -a /etc/pacman.conf > /dev/null

        print_info "Sincronizando repositorios y actualizando el sistema..."
        sudo pacman -Syu
        print_success "Repositorios Chaotic AUR configurados con éxito."
    fi
    sleep 2
}

install_bauh() {
    print_header "INSTALANDO TIENDA BAUH (AUR)"
    instalar_paquetes bauh
    print_success "Instalación de Bauh finalizada."
    sleep 2
}

install_gaming_nvidia() {
    print_header "INSTALANDO UTILIDADES GAMING (NVIDIA)"
    echo -e "1\ns" | sudo pacman -S --needed steam || true
    instalar_paquetes dxvk protonplus spotify discord gamemode proton-ge-custom-bin gedit flatpak
    print_success "Herramientas Gaming instaladas."
    sleep 2
}

install_gaming_intel() {
    print_header "INSTALANDO UTILIDADES GAMING (INTEL)"
    echo -e "7\ns" | sudo pacman -S --needed steam || true
    instalar_paquetes dxvk protonplus spotify discord gamemode proton-ge-custom-bin gedit flatpak
    print_success "Herramientas Gaming instaladas."
    sleep 2
}

install_gaming_amd() {
    print_header "INSTALANDO UTILIDADES GAMING (AMD)"
    echo -e "12\ns" | sudo pacman -S --needed steam || true
    instalar_paquetes dxvk protonplus spotify discord gamemode proton-ge-custom-bin gedit flatpak
    print_success "Herramientas Gaming instaladas."
    sleep 2
}

# ==================================================== #
#                   MENÚ INTERACTIVO                   #
# ==================================================== #

print_info "Revisando configuración de repositorios Chaotic AUR..."
install_chaotic_aur

while true; do
    print_title
    
    echo -e "${C_WHITE}Selecciona una de las opciones para configurar tu entorno:${C_DEF}\n"
    
    echo -e "  ${C_CYAN}1)${C_DEF} 🐚 Terminal Bonita (ZSH + Starship + Plugins)"
    echo -e "  ${C_CYAN}2)${C_DEF} 🪟 Instalar Entorno Hyprland (Herramientas)"
    echo -e "  ${C_CYAN}3)${C_DEF} 🪟 Instalar Entorno Niri (Herramientas)"
    echo -e "  ${C_CYAN}4)${C_DEF} 🎮 Utilidades Gaming (Steam, spotify, discord) ${C_GREEN}[NVIDIA]${C_DEF}"
    echo -e "  ${C_CYAN}5)${C_DEF} 🎮 Utilidades Gaming (Steam, spotify, discord) ${C_BLUE}[INTEL]${C_DEF}"
    echo -e "  ${C_CYAN}6)${C_DEF} 🎮 Utilidades Gaming (Steam, spotify, discord) ${C_RED}[AMD]${C_DEF}"
    echo -e "  ${C_CYAN}7)${C_DEF} 📦 Agregar repositorios Chaotic AUR"
    echo -e "  ${C_CYAN}8)${C_DEF} 🛍️ Instalar Tienda Bauh"
    echo -e "  ${C_CYAN}9)${C_DEF} ✨ Instalar TODO ${C_YELLOW}(Terminal, Hyprland, Niri, Bauh)${C_DEF}"
    echo -e "  ${C_RED}0)${C_DEF} ❌ Salir\n"
    
    echo -e "${C_CYAN}──────────────────────────────────────────────────────────────${C_DEF}"
    echo -ne "${C_WHITE}👉 Ingresa una opción: ${C_DEF}"
    read opcion

    case $opcion in
        1) install_zsh_starship ;;
        2) install_hyprland ;;
        3) install_niri ;;
        4) install_gaming_nvidia ;;
        5) install_gaming_intel ;;
        6) install_gaming_amd ;;
        7) install_chaotic_aur ;;
        8) install_bauh ;;
        9) 
           install_zsh_starship
           install_hyprland
           install_niri
           install_bauh
           print_success "¡El sistema global se configuró con éxito!"
           print_warning "Nota: Ejecuta módulos de Gaming por separado según tus necesidades."
           sleep 4
           ;;
        0) 
           print_info "Saliendo de la configuración de The-Gekko. ¡Hasta luego!"
           exit 0 
           ;;
        *) 
           print_warning "Opción no válida. Por favor, selecciona un número del 0 al 9."
           sleep 2
           ;;
    esac
done
