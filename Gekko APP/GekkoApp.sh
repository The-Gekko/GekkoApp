#!/bin/bash

# Terminar la ejecución si hay un error crítico
set -e

# ==================================================== #
#                       COLORES                        #
# ==================================================== #
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

# ==================================================== #
#                 FUNCIONES AUXILIARES                 #
# ==================================================== #

instalar_paquetes() {
    local paquetes=("$@")
    local faltantes=()

    for pkg in "${paquetes[@]}"; do
        if ! pacman -Qq "$pkg" >/dev/null 2>&1; then
            faltantes+=("$pkg")
        fi
    done

    if [ ${#faltantes[@]} -eq 0 ]; then
        print_success "Todos estos paquetes ya están instalados. No haremos nada, saltando..."
    else
        echo -e "${C_YELLOW}📦 Instalando paquetes faltantes:${C_DEF} ${faltantes[*]}"
        sudo pacman -S --needed --noconfirm "${faltantes[@]}"
    fi
}

desinstalar_paquetes() {
    local paquetes=("$@")
    local a_eliminar=()

    for pkg in "${paquetes[@]}"; do
        if pacman -Qq "$pkg" >/dev/null 2>&1; then
            a_eliminar+=("$pkg")
        fi
    done

    if [ ${#a_eliminar[@]} -eq 0 ]; then
        print_success "Esos paquetes ya no se encuentran en tu sistema. Saltando desinstalación..."
    else
        echo -e "${C_RED}🗑️ Desinstalando paquetes innecesarios:${C_DEF} ${a_eliminar[*]}"
        sudo pacman -Rns --noconfirm "${a_eliminar[@]}"
    fi
}

configurar_fastfetch() {
    print_info "Aplicando el tema universal de Fastfetch..."
    mkdir -p ~/.config/fastfetch

    cat << EOF > ~/.config/fastfetch/config.jsonc
{
  "\$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/logo.png",
  "logo": {
    "type": "kitty-direct",
    "source": "${HOME}/.config/fastfetch/Anime Render.png",
    "height": 20,
    "width": 30,
    "padding": {
      "top": 4,
      "left": 0,
      "right": 0
    }
  },
  "display": {
    "separator": " ► ",
    "bar": {
      "char": {
        "elapsed": "",
        "total": ""
      },
      "width": 15
    },
    "percent": {
      "type": 2
    }
  },
  "modules": [
    "break",
    {
      "type": "title",
      "format": " 👑 {1}@{2}",
      "keyColor": "35"
    },
    {
      "type": "custom",
      "format": "───────────────────────────────────────────────────",
      "outputColor": "33"
    },
    {
      "type": "custom",
      "format": " ✦ ✦ ✦  SYSTEM  ✦ ✦ ✦",
      "outputColor": "5"
    },
    {
      "type": "os",
      "key": " 🧩 OS  ",
      "keyColor": "yellow"
    },
    {
      "type": "kernel",
      "key": " ⚙️ KRNL",
      "keyColor": "yellow"
    },
    {
      "type": "packages",
      "key": " 📦 PKGS",
      "keyColor": "yellow"
    },
    {
      "type": "shell",
      "key": " 🐚 SH  ",
      "keyColor": "yellow"
    },
    {
      "type": "custom",
      "format": " ✦ ✦ ✦  DESKTOP ✦ ✦ ✦",
      "outputColor": "5"
    },
    {
      "type": "de",
      "key": " 🪟 DE  ",
      "keyColor": "blue"
    },
    {
      "type": "wm",
      "key": " 🧭 WM  ",
      "keyColor": "blue"
    },
    {
      "type": "lm",
      "key": " 🔐 LM  ",
      "keyColor": "blue"
    },
    {
      "type": "wmtheme",
      "key": " 🎨 THM ",
      "keyColor": "blue"
    },
    {
      "type": "icons",
      "key": " 🧩 ICO ",
      "keyColor": "blue",
      "format": "{1}"
    },
    {
      "type": "terminal",
      "key": " 🖥️ TERM",
      "keyColor": "blue"
    },
    {
      "type": "custom",
      "format": " ✦ ✦ ✦  Hardware  ✦ ✦ ✦",
      "outputColor": "5"
    },
    {
      "type": "host",
      "key": " 💻 HST ",
      "keyColor": "green"
    },
    {
      "type": "cpu",
      "key": " 🧠 CPU ",
      "keyColor": "green",
      "format": "{1}"
    },
    {
      "type": "gpu",
      "key": " 🎮 GPU ",
      "keyColor": "green"
    },
    {
      "type": "disk",
      "key": " 💾 DSK ",
      "keyColor": "green"
    },
    {
      "type": "memory",
      "key": " 🧬 RAM ",
      "keyColor": "green"
    },
    {
      "type": "swap",
      "key": " 🔄 SWP ",
      "keyColor": "green"
    },
    {
      "type": "custom",
      "format": " ✦ ✦ ✦  Media  ✦ ✦ ✦",
      "outputColor": "5"
    },
    {
      "type": "sound",
      "key": " 🔊 VOL ",
      "keyColor": "cyan"
    },
    {
      "type": "player",
      "key": " 🎵 PLY ",
      "keyColor": "cyan"
    },
    {
      "type": "media",
      "key": " 🎬 MED ",
      "keyColor": "cyan"
    }
  ]
}
EOF
    print_success "Configuración .jsonc dinámica de Fastfetch generada."
    
    if [ ! -f "$HOME/.config/fastfetch/Anime Render.png" ]; then
        if [ -f "./Anime Render.png" ]; then
            cp "./Anime Render.png" "$HOME/.config/fastfetch/"
            print_success "Imagen 'Anime Render.png' copiada a ~/.config/fastfetch/"
        else
            print_warning "Recuerda copiar tu imagen 'Anime Render.png' en ~/.config/fastfetch/ para ver el logo estilo kitty."
        fi
    fi
}

# ==================================================== #
#                 FUNCIONES DE INSTALACIÓN             #
# ==================================================== #

install_zsh_starship() {
    print_header "INSTALANDO ZSH Y TERMINAL BONITA (by GekkoApp)"
    sleep 1
    
    instalar_paquetes kitty git zsh nano curl eza fastfetch fzf ttf-jetbrains-mono-nerd
    fc-cache -fv

    print_info "Evaluando instalación de Starship..."
    if ! command -v starship &> /dev/null; then
        print_info "Instalando Starship..."
        curl -sS https://starship.rs/install.sh -o install.sh
        sh install.sh -y
        rm install.sh
    else
        print_success "Starship ya está instalado en el sistema."
    fi

    print_info "Instalando complementos de zsh..."
    mkdir -p ~/.zsh
    [ ! -d ~/.zsh/zsh-autosuggestions ] && git clone https://github.com/zsh-users/zsh-autosuggestions ~/.zsh/zsh-autosuggestions || print_success "autosuggestions ya clonado."
    [ ! -d ~/.zsh/zsh-syntax-highlighting ] && git clone https://github.com/zsh-users/zsh-syntax-highlighting ~/.zsh/zsh-syntax-highlighting || print_success "syntax-highlighting ya clonado."
    [ ! -d ~/.zsh/zsh-history-substring-search ] && git clone https://github.com/zsh-users/zsh-history-substring-search ~/.zsh/zsh-history-substring-search || print_success "history-substring-search ya clonado."
    [ ! -d ~/.zsh/zsh-completions ] && git clone https://github.com/zsh-users/zsh-completions ~/.zsh/zsh-completions || print_success "completions ya clonado."
    [ ! -d ~/.zsh/z ] && git clone https://github.com/rupa/z.git ~/.zsh/z || print_success "z (jump) ya clonada."

    cat << 'EOF' > ~/.zshrc
# =====================================
# HISTORIAL
# =====================================
HISTFILE=~/.zsh_history
HISTSIZE=100000
SAVEHIST=100000
setopt APPEND_HISTORY
setopt INC_APPEND_HISTORY
setopt SHARE_HISTORY
setopt HIST_IGNORE_ALL_DUPS
setopt HIST_IGNORE_SPACE
setopt HIST_REDUCE_BLANKS
setopt HIST_VERIFY

# =====================================
# COMPLETION
# =====================================
autoload -Uz compinit
fpath=(~/.zsh/zsh-completions/src $fpath)
compinit -d ~/.zcompdump
zstyle ':completion:*' matcher-list 'm:{a-z}={A-Za-z}' 'r:|[._-]=* r:|=*'
setopt AUTO_MENU
setopt COMPLETE_IN_WORD
setopt ALWAYS_TO_END

# =====================================
# ALIASES
# =====================================
alias deit="bash <(curl -sL https://raw.githubusercontent.com/iJoseG/Mscripts/refs/heads/main/datetoday.sh)"
alias actrepo="bash <(curl -sL https://raw.githubusercontent.com/iJoseG/Mscripts/refs/heads/main/actrepo.sh)"
alias l='eza -l --icons --color=auto --group-directories-first'
alias ls='eza --icons --color=auto --group-directories-first'
alias ll='eza -l --icons --color=auto --group-directories-first'
alias la='eza -la --icons --color=auto --group-directories-first'
alias lt='eza --tree --icons --color=auto --group-directories-first'

# =====================================
# PROMPT Y PLUGINS
# =====================================
eval "$(starship init zsh)"

source ~/.zsh/zsh-autosuggestions/zsh-autosuggestions.zsh
source ~/.zsh/zsh-history-substring-search/zsh-history-substring-search.zsh
bindkey '^[[A' history-substring-search-up
bindkey '^[[B' history-substring-search-down

[ -f /usr/share/fzf/key-bindings.zsh ] && source /usr/share/fzf/key-bindings.zsh
[ -f /usr/share/fzf/completion.zsh ] && source /usr/share/fzf/completion.zsh

source ~/.zsh/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh
source ~/.zsh/z/z.sh

# =====================================
# KEYBINDINGS
# =====================================
stty -ixon
bindkey '^Q' kill-whole-line
bindkey '^U' backward-kill-line
bindkey '^K' kill-line
bindkey '^[[1;5C' forward-word
bindkey '^[[1;5D' backward-word

# =====================================
# FASTFETCH
# =====================================
[[ $- == *i* ]] && fastfetch
EOF

    print_info "Estableciendo preset de starship..."
    mkdir -p ~/.config
    starship preset jetpack -o ~/.config/starship.toml

    configurar_fastfetch
    sudo chsh -s /bin/zsh $USER

    print_success "Instalación de Terminal Finalizada... by GekkoApp"
    print_warning "Reinicia la sesión o ejecuta: exec zsh para aplicar cambios."
    sleep 2
}

install_hyprland() {
    print_header "INSTALANDO PRESET HYPRLAND"
    instalar_paquetes gedit electron gnome-keyring polkit-gnome xdg-desktop-portal-gtk xwayland-satellite nwg-look blueman ibus unzip colord bolt flatpak gnome-disk-utility gvfs-mtp gvfs-gphoto2 libmtp nautilus fuse2 ddcutil i2c-dev
    desinstalar_paquetes dolphin polkit-kde-agent wofi 
    print_success "Evaluación de dependencias de Hyprland finalizada."
    sleep 2
}

install_niri() {
    print_header "INSTALANDO PRESET NIRI"
    instalar_paquetes gedit xdg-desktop-portal-gnome gnome-keyring polkit-gnome xdg-desktop-portal-gtk electron xwayland-satellite gnome-disk-utility qt5-wayland qt6-wayland nwg-look flatpak blueman gvfs-mtp gvfs-gphoto2 libmtp ibus unzip colord bolt fuse2 ddcutil i2c-dev playerctl gpsd dconf-editor 
    desinstalar_paquetes mako swaybg swayidle swaylock waybar 
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

install_gaming_nvidia() {
    print_header "INSTALANDO UTILIDADES GAMING (NVIDIA)"
    instalar_paquetes dxvk protonplus spotify discord gamemode proton-ge-custom-bin gedit
    echo -e "1" | sudo pacman -S --needed steam || true
    print_success "Herramientas Gaming instaladas."
    sleep 2
}

install_gaming_intel() {
    print_header "INSTALANDO UTILIDADES GAMING (INTEL)"
    instalar_paquetes dxvk protonplus spotify discord gamemode proton-ge-custom-bin gedit
    echo -e "7" | sudo pacman -S --needed steam || true
    print_success "Herramientas Gaming instaladas."
    sleep 2
}

install_gaming_amd() {
    print_header "INSTALANDO UTILIDADES GAMING (AMD)"
    instalar_paquetes dxvk protonplus spotify discord gamemode proton-ge-custom-bin gedit
    echo -e "12" | sudo pacman -S --needed steam || true
    print_success "Herramientas Gaming instaladas."
    sleep 2
}

# ==================================================== #
#                   MENÚ INTERACTIVO                   #
# ==================================================== #

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
    echo -e "  ${C_CYAN}8)${C_DEF} ✨ Instalar TODO ${C_YELLOW}(Sin módulos Gaming/Chaotic AUR)${C_DEF}"
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
        8) 
           install_zsh_starship
           install_hyprland
           install_niri
           print_success "¡El sistema global se configuró con éxito!"
           print_warning "Nota: Ejecuta módulos de Gaming y Chaotic AUR por separado según tus necesidades."
           sleep 4
           ;;
        0) 
           print_info "Saliendo de la configuración de The-Gekko. ¡Hasta luego!"
           exit 0 
           ;;
        *) 
           print_warning "Opción no válida. Por favor, selecciona un número del 0 al 8."
           sleep 2
           ;;
    esac
done
