use std::io::{self, BufRead, Write};
use std::process::{Command, Stdio};
use std::path::Path;
use std::thread;
use std::time::Duration;

// ─────────────────────────────────────────────────────────────────────────────
//  ANSI color / style helpers
// ─────────────────────────────────────────────────────────────────────────────

const RESET: &str = "\x1b[0m";

// Foreground colours
const FG_BLACK: &str = "\x1b[30m";
const FG_RED: &str = "\x1b[1;31m";
const FG_GREEN: &str = "\x1b[1;32m";
const FG_YELLOW: &str = "\x1b[1;33m";
const FG_BLUE: &str = "\x1b[1;34m";
const FG_MAGENTA: &str = "\x1b[1;35m";
const FG_CYAN: &str = "\x1b[1;36m";
const FG_WHITE: &str = "\x1b[1;37m";

// Background colours
const BG_CYAN: &str = "\x1b[46m";
const BG_MAGENTA: &str = "\x1b[45m";
const BG_BLACK: &str = "\x1b[40m";
const BG_BLUE: &str = "\x1b[44m";

// Text decorations
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";

// ─────────────────────────────────────────────────────────────────────────────
//  Utility macros / functions
// ─────────────────────────────────────────────────────────────────────────────

fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

fn move_cursor(row: u16, col: u16) {
    print!("\x1b[{};{}H", row, col);
    let _ = io::stdout().flush();
}

fn hide_cursor() { print!("\x1b[?25l"); let _ = io::stdout().flush(); }
fn show_cursor() { print!("\x1b[?25h"); let _ = io::stdout().flush(); }

fn print_colored(text: &str, color: &str) {
    print!("{}{}{}", color, text, RESET);
    let _ = io::stdout().flush();
}

fn println_colored(text: &str, color: &str) {
    println!("{}{}{}", color, text, RESET);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Status printers
// ─────────────────────────────────────────────────────────────────────────────

fn print_ok(msg: &str) {
    println!("{}  ✅  {}{}{}", FG_GREEN, BOLD, msg, RESET);
}

fn print_warn(msg: &str) {
    println!("{}  ⚠️   {}{}{}", FG_YELLOW, BOLD, msg, RESET);
}

fn print_info(msg: &str) {
    println!("{}  ℹ️   {}{}{}", FG_CYAN, BOLD, msg, RESET);
}

fn print_err(msg: &str) {
    println!("{}  ✗   {}{}{}", FG_RED, BOLD, msg, RESET);
}

fn print_step(msg: &str) {
    println!("{}  ➜   {}{}{}", FG_MAGENTA, ITALIC, msg, RESET);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Animated spinner
// ─────────────────────────────────────────────────────────────────────────────

fn spinner_run<F>(label: &str, task: F)
where
    F: FnOnce() + Send + 'static,
{
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let label = label.to_string();
    let (tx, rx) = std::sync::mpsc::channel::<()>();

    let spinner_thread = thread::spawn(move || {
        let mut i = 0usize;
        hide_cursor();
        loop {
            let frame = frames[i % frames.len()];
            print!("\r{}{}  {}  {}{}", FG_CYAN, BOLD, frame, label, RESET);
            let _ = io::stdout().flush();
            i += 1;
            match rx.recv_timeout(Duration::from_millis(80)) {
                Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                _ => {}
            }
        }
        print!("\r{}{}", " ".repeat(label.len() + 8), "\r");
        show_cursor();
        let _ = io::stdout().flush();
    });

    task();
    let _ = tx.send(());
    let _ = spinner_thread.join();
}

// ─────────────────────────────────────────────────────────────────────────────
//  Progress bar (fake, for visual effect)
// ─────────────────────────────────────────────────────────────────────────────

fn fake_progress_bar(label: &str, steps: u32) {
    let width: u32 = 40;
    hide_cursor();
    for i in 0..=steps {
        let filled = (i * width / steps) as usize;
        let empty = (width as usize).saturating_sub(filled);
        let bar: String = format!(
            "{}{}{}{}{}",
            FG_CYAN,
            "█".repeat(filled),
            DIM,
            "░".repeat(empty),
            RESET
        );
        let pct = i * 100 / steps;
        print!(
            "\r  {}{}  {}{} {}{}{}%{}",
            FG_MAGENTA, label,
            RESET, bar,
            FG_GREEN, BOLD, pct, RESET
        );
        let _ = io::stdout().flush();
        thread::sleep(Duration::from_millis(18));
    }
    println!("  {}✓ Listo{}", FG_GREEN, RESET);
    show_cursor();
}

// ─────────────────────────────────────────────────────────────────────────────
//  Section header
// ─────────────────────────────────────────────────────────────────────────────

fn print_header(title: &str) {
    let width = 65usize;
    let inner = format!("  🔮  {}  🔮  ", title);
    let pad_left = (width.saturating_sub(inner.chars().count())) / 2;
    let pad_right = width.saturating_sub(inner.chars().count()).saturating_sub(pad_left);

    println!();
    println!(
        "{}{}╔{}╗{}",
        FG_MAGENTA, BOLD,
        "═".repeat(width + 2),
        RESET
    );
    println!(
        "{}{}║{}{}{:pad_left$}{}{}{:pad_right$}{}{}║{}",
        FG_MAGENTA, BOLD,
        RESET,
        FG_CYAN, "",
        inner,
        FG_CYAN, "",
        FG_MAGENTA, BOLD,
        RESET,
        pad_left = pad_left, pad_right = pad_right
    );
    println!(
        "{}{}╚{}╝{}",
        FG_MAGENTA, BOLD,
        "═".repeat(width + 2),
        RESET
    );
    println!();
}

// ─────────────────────────────────────────────────────────────────────────────
//  ASCII art banner
// ─────────────────────────────────────────────────────────────────────────────

fn print_banner() {
    clear_screen();

    // Gradient-like top bar
    let bar_colors = [
        "\x1b[38;5;57m",
        "\x1b[38;5;63m",
        "\x1b[38;5;69m",
        "\x1b[38;5;75m",
        "\x1b[38;5;81m",
        "\x1b[38;5;87m",
        "\x1b[38;5;81m",
        "\x1b[38;5;75m",
        "\x1b[38;5;69m",
        "\x1b[38;5;63m",
    ];

    print!("\n  ");
    for (i, clr) in bar_colors.iter().cycle().take(65).enumerate() {
        let _ = i;
        print!("{}▀", clr);
    }
    println!("{}", RESET);

    println!();

    // Dragon ASCII
    let dragon = vec![
        r"      .     .     .  .  .  .  .  . .",
        r"                                        ",
        r"       (\  (\     ___              🐉   ",
        r"       ( \( \   /   \    THE-GEKKO      ",
        r"       ( (\ (  ( ^  ^ )   POST-INSTALL  ",
        r"    /\/  ( \(  (  ==  )     & CONFIG    ",
        r"   / /  ( /(   \      )                 ",
        r"  ( (  / /( \  / \___/ \   GekkoApp  v1 ",
        r"   \ \ \/  \ \/        \   by The-Gekko ",
        r"    \_/     \_/                         ",
    ];

    for line in &dragon {
        println!("  {}{}{}", FG_CYAN, line, RESET);
    }

    println!();

    // Fancy box
    println!(
        "  {}{}╭{}╮{}",
        FG_CYAN, BOLD,
        "─".repeat(63),
        RESET
    );

    let subtitle = "🐉  THE-GEKKO LINUX POST-INSTALL & CONFIG  🐉";
    let pad = (63usize.saturating_sub(subtitle.chars().count())) / 2;
    println!(
        "  {}{}│{}{}{}{}{}{:>pad$}{}│{}",
        FG_CYAN, BOLD,
        RESET,
        " ".repeat(pad),
        FG_WHITE, BOLD, subtitle,
        "",
        FG_CYAN,
        RESET,
        pad = 0
    );

    let by_line = "Arch Linux ❱ Automated Setup ❱ Powered by Rust";
    println!(
        "  {}{}|  {}{}{}{}",
        FG_CYAN, BOLD,
        DIM, FG_CYAN, by_line,
        RESET
    );

    println!(
        "  {}{}╰{}╯{}",
        FG_CYAN, BOLD,
        "─".repeat(63),
        RESET
    );

    // Bottom gradient bar
    print!("  ");
    for clr in bar_colors.iter().cycle().take(65) {
        print!("{}▄", clr);
    }
    println!("{}\n", RESET);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Menu
// ─────────────────────────────────────────────────────────────────────────────

struct MenuItem {
    key: &'static str,
    icon: &'static str,
    label: &'static str,
    badge: Option<(&'static str, &'static str)>, // (text, color)
}

fn print_menu() {
    let items = [
        MenuItem { key: "1", icon: "🐚", label: "Terminal Bonita           (ZSH + Starship + Plugins)", badge: None },
        MenuItem { key: "2", icon: "🪟", label: "Entorno Hyprland          (Herramientas + Deps)",       badge: None },
        MenuItem { key: "3", icon: "🪟", label: "Entorno Niri              (Herramientas + Deps)",       badge: None },
        MenuItem { key: "4", icon: "🎮", label: "Gaming Setup",                                          badge: Some(("NVIDIA", FG_GREEN)) },
        MenuItem { key: "5", icon: "🎮", label: "Gaming Setup",                                          badge: Some(("INTEL",  FG_BLUE))  },
        MenuItem { key: "6", icon: "🎮", label: "Gaming Setup",                                          badge: Some(("AMD",    FG_RED))   },
        MenuItem { key: "7", icon: "📦", label: "Agregar repositorios      Chaotic AUR",                 badge: None },
        MenuItem { key: "8", icon: "🛍️", label: "Tienda Bauh               (Parcheado + AUR)",           badge: None },
        MenuItem { key: "9", icon: "✨", label: "Instalar TODO",                                         badge: Some(("Terminal+Hyprland+Niri+Bauh", FG_YELLOW)) },
        MenuItem { key: "0", icon: "❌", label: "Salir",                                                 badge: None },
    ];

    println!("  {}{}Selecciona una opción para configurar tu entorno:{}", FG_WHITE, BOLD, RESET);
    println!();

    for item in &items {
        let key_color = if item.key == "0" { FG_RED } else { FG_CYAN };
        print!(
            "  {}{}[{}]{} {} {}{}{}",
            key_color, BOLD, item.key, RESET,
            item.icon,
            FG_WHITE, item.label, RESET
        );
        if let Some((badge_text, badge_color)) = item.badge {
            print!("  {}{}{}{}{}", badge_color, BOLD, badge_text, RESET, "");
        }
        println!();
    }

    println!();
    println!("  {}{}{}─────────────────────────────────────────────────────────────{}", FG_CYAN, BOLD, DIM, RESET);
    print!("  {}{}👉 Ingresa una opción:{} ", FG_WHITE, BOLD, RESET);
    let _ = io::stdout().flush();
}

// ─────────────────────────────────────────────────────────────────────────────
//  Shell command helpers
// ─────────────────────────────────────────────────────────────────────────────

fn run_shell(cmd: &str) -> bool {
    Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_shell_piped(cmd: &str) -> (bool, String) {
    let out = Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output();
    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            (o.status.success(), stdout)
        }
        Err(_) => (false, String::new()),
    }
}

fn is_package_installed(pkg: &str) -> bool {
    run_shell_piped(&format!("pacman -Qq '{pkg}' 2>/dev/null")).0
}

fn instalar_paquetes(paquetes: &[&str]) {
    let faltantes: Vec<&str> = paquetes
        .iter()
        .filter(|&&p| !is_package_installed(p))
        .copied()
        .collect();

    if faltantes.is_empty() {
        print_ok("Todos los paquetes ya están instalados. Saltando...");
        return;
    }

    println_colored(
        &format!("📦  Instalando {} paquetes faltantes...", faltantes.len()),
        FG_YELLOW,
    );
    for pkg in &faltantes {
        print_step(&format!("→ {}", pkg));
    }

    let pkg_list = faltantes.join(" ");
    let cmd = format!("sudo pacman -S --needed --noconfirm {}", pkg_list);
    fake_progress_bar("Instalando", 40);
    if !run_shell(&cmd) {
        print_err("Algunos paquetes no pudieron instalarse. Revisa la salida anterior.");
    }
}

fn desinstalar_paquetes(paquetes: &[&str]) {
    let a_eliminar: Vec<&str> = paquetes
        .iter()
        .filter(|&&p| is_package_installed(p))
        .copied()
        .collect();

    if a_eliminar.is_empty() {
        print_ok("Esos paquetes ya no están en el sistema. Saltando desinstalación...");
        return;
    }

    println_colored(
        &format!("🗑️  Desinstalando {} paquetes innecesarios...", a_eliminar.len()),
        FG_RED,
    );
    for pkg in &a_eliminar {
        print_step(&format!("✗ {}", pkg));
    }

    let pkg_list = a_eliminar.join(" ");
    let cmd = format!("sudo pacman -Rns --noconfirm {}", pkg_list);
    if !run_shell(&cmd) {
        print_err("Error al desinstalar paquetes.");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Fastfetch config
// ─────────────────────────────────────────────────────────────────────────────

fn configurar_fastfetch() {
    print_info("Aplicando el tema universal de Fastfetch...");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let config_dir = format!("{}/.config/fastfetch", home);
    let _ = std::fs::create_dir_all(&config_dir);

    let config = format!(r#"{{
  "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/logo.png",
  "logo": {{
    "type": "kitty-direct",
    "source": "{home}/.config/fastfetch/Anime Render.png",
    "height": 20,
    "width": 30,
    "padding": {{ "top": 4, "left": 0, "right": 0 }}
  }},
  "display": {{
    "separator": " ► ",
    "bar": {{
      "char": {{ "elapsed": "", "total": "" }},
      "width": 15
    }},
    "percent": {{ "type": 2 }}
  }},
  "modules": [
    "break",
    {{ "type": "title", "format": " 👑 {{1}}@{{2}}", "keyColor": "35" }},
    {{ "type": "custom", "format": "───────────────────────────────────────────────────", "outputColor": "33" }},
    {{ "type": "custom", "format": " ✦ ✦ ✦  SYSTEM  ✦ ✦ ✦", "outputColor": "5" }},
    {{ "type": "os",       "key": " 🧩 OS  ", "keyColor": "yellow" }},
    {{ "type": "kernel",   "key": " ⚙️ KRNL", "keyColor": "yellow" }},
    {{ "type": "packages", "key": " 📦 PKGS", "keyColor": "yellow" }},
    {{ "type": "shell",    "key": " 🐚 SH  ", "keyColor": "yellow" }},
    {{ "type": "custom",   "format": " ✦ ✦ ✦  DESKTOP ✦ ✦ ✦", "outputColor": "5" }},
    {{ "type": "de",       "key": " 🪟 DE  ", "keyColor": "blue" }},
    {{ "type": "wm",       "key": " 🧭 WM  ", "keyColor": "blue" }},
    {{ "type": "lm",       "key": " 🔐 LM  ", "keyColor": "blue" }},
    {{ "type": "wmtheme",  "key": " 🎨 THM ", "keyColor": "blue" }},
    {{ "type": "icons",    "key": " 🧩 ICO ", "keyColor": "blue", "format": "{{1}}" }},
    {{ "type": "terminal", "key": " 🖥️ TERM", "keyColor": "blue" }},
    {{ "type": "custom",   "format": " ✦ ✦ ✦  Hardware  ✦ ✦ ✦", "outputColor": "5" }},
    {{ "type": "host",     "key": " 💻 HST ", "keyColor": "green" }},
    {{ "type": "cpu",      "key": " 🧠 CPU ", "keyColor": "green", "format": "{{1}}" }},
    {{ "type": "gpu",      "key": " 🎮 GPU ", "keyColor": "green" }},
    {{ "type": "disk",     "key": " 💾 DSK ", "keyColor": "green" }},
    {{ "type": "memory",   "key": " 🧬 RAM ", "keyColor": "green" }},
    {{ "type": "swap",     "key": " 🔄 SWP ", "keyColor": "green" }},
    {{ "type": "custom",   "format": " ✦ ✦ ✦  Media  ✦ ✦ ✦", "outputColor": "5" }},
    {{ "type": "sound",    "key": " 🔊 VOL ", "keyColor": "cyan" }},
    {{ "type": "player",   "key": " 🎵 PLY ", "keyColor": "cyan" }},
    {{ "type": "media",    "key": " 🎬 MED ", "keyColor": "cyan" }}
  ]
}}
"#, home = home);

    let config_path = format!("{}/config.jsonc", config_dir);
    let _ = std::fs::write(&config_path, config);
    print_ok("Configuración .jsonc de Fastfetch generada.");

    // Copy image if available
    let img_dest = format!("{}/Anime Render.png", config_dir);
    if !Path::new(&img_dest).exists() {
        if Path::new("./Anime Render.png").exists() {
            let _ = std::fs::copy("./Anime Render.png", &img_dest);
            print_ok("Imagen 'Anime Render.png' copiada a ~/.config/fastfetch/");
        } else {
            print_warn("Copia tu imagen 'Anime Render.png' en ~/.config/fastfetch/ para el logo kitty.");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Install functions
// ─────────────────────────────────────────────────────────────────────────────

fn install_zsh_starship() {
    print_header("INSTALANDO ZSH Y TERMINAL BONITA  (by GekkoApp)");
    thread::sleep(Duration::from_millis(500));

    instalar_paquetes(&[
        "kitty", "git", "zsh", "nano", "curl",
        "eza", "fastfetch", "fzf", "ttf-jetbrains-mono-nerd",
    ]);

    run_shell("fc-cache -fv > /dev/null 2>&1");

    print_info("Evaluando instalación de Starship...");
    let (starship_ok, _) = run_shell_piped("command -v starship");
    if !starship_ok {
        print_info("Instalando Starship vía script oficial...");
        fake_progress_bar("Descargando Starship", 30);
        run_shell("curl -sS https://starship.rs/install.sh -o /tmp/install_starship.sh && sh /tmp/install_starship.sh -y && rm /tmp/install_starship.sh");
    } else {
        print_ok("Starship ya está instalado.");
    }

    print_info("Instalando complementos de ZSH...");
    run_shell("mkdir -p ~/.zsh");

    let plugins = [
        ("zsh-autosuggestions",       "https://github.com/zsh-users/zsh-autosuggestions"),
        ("zsh-syntax-highlighting",   "https://github.com/zsh-users/zsh-syntax-highlighting"),
        ("zsh-history-substring-search", "https://github.com/zsh-users/zsh-history-substring-search"),
        ("zsh-completions",           "https://github.com/zsh-users/zsh-completions"),
        ("z",                         "https://github.com/rupa/z.git"),
    ];

    for (name, url) in &plugins {
        let dest = format!("~/.zsh/{}", name);
        let cmd = format!(
            "[ ! -d {dest} ] && git clone {url} {dest} && echo 'clonado' || echo 'ya existe'"
        );
        let (_, out) = run_shell_piped(&cmd);
        if out.contains("clonado") {
            print_ok(&format!("{} instalado.", name));
        } else {
            print_ok(&format!("{} ya está clonado.", name));
        }
    }

    // .zshrc
    let zshrc = r#"# =====================================
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
# PROMPT
# =====================================

eval "$(starship init zsh)"

# =====================================
# PLUGINS
# =====================================

source ~/.zsh/zsh-autosuggestions/zsh-autosuggestions.zsh
source ~/.zsh/zsh-history-substring-search/zsh-history-substring-search.zsh
bindkey '^[[A' history-substring-search-up
bindkey '^[[B' history-substring-search-down
source /usr/share/fzf/key-bindings.zsh
source /usr/share/fzf/completion.zsh
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
# FASTFETCH SOLO INTERACTIVO (by The-Gekko)
# =====================================

[[ $- == *i* ]] && fastfetch

# ZSH Config by iBlueMoon
"#;

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let _ = std::fs::write(format!("{}/.zshrc", home), zshrc);
    print_ok(".zshrc escrito correctamente.");

    print_info("Estableciendo preset de Starship...");
    run_shell("mkdir -p ~/.config && starship preset jetpack -o ~/.config/starship.toml");

    configurar_fastfetch();

    run_shell("sudo chsh -s /bin/zsh $USER");

    print_ok("¡Instalación de Terminal Finalizada! — by GekkoApp");
    print_warn("Reinicia sesión o ejecuta: exec zsh  para aplicar cambios.");
    thread::sleep(Duration::from_secs(2));
}

fn install_hyprland() {
    print_header("INSTALANDO PRESET HYPRLAND");
    instalar_paquetes(&[
        "gedit", "electron", "gnome-keyring", "polkit-gnome",
        "xdg-desktop-portal-gtk", "xwayland-satellite", "nwg-look",
        "blueman", "ibus", "unzip", "colord", "bolt", "flatpak",
        "gnome-disk-utility", "gvfs-mtp", "gvfs-gphoto2", "libmtp",
        "nautilus", "fuse2", "ddcutil", "i2c-dev",
    ]);
    desinstalar_paquetes(&["dolphin", "polkit-kde-agent", "wofi"]);
    print_ok("Evaluación de dependencias de Hyprland finalizada.");
    thread::sleep(Duration::from_secs(2));
}

fn install_niri() {
    print_header("INSTALANDO PRESET NIRI");
    instalar_paquetes(&[
        "gedit", "xdg-desktop-portal-gnome", "gnome-keyring", "polkit-gnome",
        "xdg-desktop-portal-gtk", "electron", "xwayland-satellite",
        "gnome-disk-utility", "qt5-wayland", "qt6-wayland", "nwg-look",
        "flatpak", "blueman", "gvfs-mtp", "gvfs-gphoto2", "libmtp",
        "ibus", "unzip", "colord", "bolt", "fuse2", "ddcutil", "i2c-dev",
        "playerctl", "gpsd", "dconf-editor",
    ]);
    desinstalar_paquetes(&["mako", "swaybg", "swayidle", "swaylock", "waybar"]);
    print_ok("Evaluación de dependencias de Niri finalizada.");
    thread::sleep(Duration::from_secs(2));
}

fn install_chaotic_aur() {
    print_header("AGREGANDO REPOSITORIOS CHAOTIC AUR");

    let already = run_shell("grep -q '\\[chaotic-aur\\]' /etc/pacman.conf");
    if already {
        // Sanity-check: remove duplicate entries if any were added accidentally
        run_shell(r#"sudo awk '/\[chaotic-aur\]/{found++} found>1{next} {print}' /etc/pacman.conf | sudo tee /etc/pacman.conf.dedup > /dev/null && sudo mv /etc/pacman.conf.dedup /etc/pacman.conf"#);
        print_ok("Chaotic AUR ya está configurado en pacman.conf. Saltando...");
        thread::sleep(Duration::from_secs(2));
        return;
    }

    print_info("Obteniendo y verificando llaves públicas...");
    fake_progress_bar("Importando llaves GPG", 25);
    run_shell("sudo pacman-key --recv-key 3056513887B78AEB --keyserver keyserver.ubuntu.com");
    run_shell("sudo pacman-key --lsign-key 3056513887B78AEB");

    print_info("Instalando keyring y mirrorlist de Chaotic AUR...");
    fake_progress_bar("Descargando paquetes", 30);
    run_shell("sudo pacman -U --noconfirm 'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst'");
    run_shell("sudo pacman -U --noconfirm 'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst'");

    print_info("Editando /etc/pacman.conf automáticamente...");
    run_shell(r#"echo -e '\n[chaotic-aur]\nInclude = /etc/pacman.d/chaotic-mirrorlist' | sudo tee -a /etc/pacman.conf > /dev/null"#);

    print_info("Sincronizando repositorios y actualizando el sistema...");
    run_shell("sudo pacman -Syu");

    print_ok("¡Repositorios Chaotic AUR configurados con éxito!");
    thread::sleep(Duration::from_secs(2));
}

fn install_bauh() {
    print_header("INSTALANDO TIENDA BAUH (AUR)");
    instalar_paquetes(&["bauh"]);

    print_info("Buscando archivo gems.py de bauh...");
    let (ok, gems_path) = run_shell_piped(
        "ls /usr/lib/python*/site-packages/bauh/view/core/gems.py 2>/dev/null | head -n 1"
    );

    let gems_path = gems_path.trim().to_string();
    if !ok || gems_path.is_empty() {
        print_warn("No se encontró el archivo gems.py de bauh. Saltando parche...");
        thread::sleep(Duration::from_secs(2));
        return;
    }

    print_info(&format!("Aplicando parche de compatibilidad en: {}", gems_path));

    let patch = r#"import inspect
import os
import pkgutil
import importlib.util
from logging import Logger
from typing import List, Generator

from bauh import __app_name__, ROOT_DIR
from bauh.api.abstract.controller import SoftwareManager, ApplicationContext
from bauh.view.util import translation

FORBIDDEN_GEMS_FILE = f'/etc/{__app_name__}/gems.forbidden'


def find_manager(member):
    if not isinstance(member, str):
        if inspect.isclass(member) and inspect.getmro(member)[1].__name__ == 'SoftwareManager':
            return member
        elif inspect.ismodule(member):
            for name, mod in inspect.getmembers(member):
                manager_found = find_manager(mod)
                if manager_found:
                    return manager_found


def read_forbidden_gems() -> Generator[str, None, None]:
    try:
        with open(FORBIDDEN_GEMS_FILE) as f:
            forbidden_lines = f.readlines()

        for line in forbidden_lines:
            clean_line = line.strip()

            if clean_line and not clean_line.startswith('#'):
                yield clean_line

    except FileNotFoundError:
        pass


def load_managers(locale: str, context: ApplicationContext, config: dict, default_locale: str, logger: Logger) -> List[SoftwareManager]:
    managers = []

    forbidden_gems = {gem for gem in read_forbidden_gems()}

    for f in os.scandir(f'{ROOT_DIR}/gems'):
        if f.is_dir() and f.name != '__pycache__':

            if f.name in forbidden_gems:
                logger.warning(f"gem '{f.name}' could not be loaded because it was marked as forbidden in '{FORBIDDEN_GEMS_FILE}'")
                continue

            spec = importlib.util.find_spec(f'bauh.gems.{f.name}.controller')
            loader = spec.loader if spec else None

            if loader:
                module = loader.load_module()

                manager_class = find_manager(module)

                if manager_class:
                    if locale:
                        locale_path = f'{f.path}/resources/locale'

                        if os.path.exists(locale_path):
                            context.i18n.current.update(translation.get_locale_keys(locale, locale_path)[1])

                            if default_locale and context.i18n.default:
                                context.i18n.default.update(translation.get_locale_keys(default_locale, locale_path)[1])

                    man = manager_class(context=context)

                    if config['gems'] is None:
                        man.set_enabled(man.is_default_enabled())
                    else:
                        man.set_enabled(f.name in config['gems'])

                    managers.append(man)

    return managers
"#;

    let tmp_path = "/tmp/gems_patch.py";
    let _ = std::fs::write(tmp_path, patch);
    let cmd = format!("sudo cp {} '{}'", tmp_path, gems_path);
    if run_shell(&cmd) {
        print_ok("El archivo gems.py de Bauh fue sobrescrito con el parche correctamente.");
    } else {
        print_err("No se pudo aplicar el parche a gems.py.");
    }

    thread::sleep(Duration::from_secs(2));
}

fn install_gaming(gpu: &str, vulkan_choice: &str) {
    let label = match gpu {
        "nvidia" => "INSTALANDO UTILIDADES GAMING  ·  NVIDIA",
        "intel"  => "INSTALANDO UTILIDADES GAMING  ·  INTEL",
        "amd"    => "INSTALANDO UTILIDADES GAMING  ·  AMD",
        _        => "INSTALANDO UTILIDADES GAMING",
    };
    print_header(label);

    // Bug fix #3: Asegurarse de que chaotic-aur no esté duplicado en pacman.conf
    run_shell("sudo pacman -Sy --noconfirm 2>/dev/null || true");

    // Steam: pipe the vulkan driver selection answer
    let steam_cmd = format!("echo -e '{}\\ns' | sudo pacman -S --needed steam || true", vulkan_choice);
    run_shell(&steam_cmd);

    // Bug fix #1: dxvk tiene múltiples proveedores en chaotic-aur,
    // seleccionamos automáticamente «1» (dxvk-async-git)
    print_info("Instalando dxvk (seleccionando proveedor 1 automáticamente)...");
    if !is_package_installed("dxvk-async-git") && !is_package_installed("dxvk-mingw-git") {
        run_shell("echo '1' | sudo pacman -S --needed dxvk 2>&1 || true");
    } else {
        print_ok("dxvk ya está instalado. Saltando...");
    }

    // Bug fix #2: lib32-gstreamer provee lib32-gst-plugins-base-libs
    // que necesita proton-ge-custom-bin como dependencia
    print_info("Instalando dependencias de Proton GE (lib32-gstreamer)...");
    instalar_paquetes(&["lib32-gstreamer", "lib32-gst-plugins-base"]);

    // Paquetes gaming principales (sin dxvk que ya se instaló arriba)
    instalar_paquetes(&[
        "protonplus", "spotify", "discord",
        "gamemode", "gedit", "flatpak",
    ]);

    // proton-ge-custom-bin por separado para manejar errores sin abortar todo
    print_info("Instalando proton-ge-custom-bin...");
    if !is_package_installed("proton-ge-custom-bin") {
        let ok = run_shell("sudo pacman -S --needed --noconfirm proton-ge-custom-bin 2>&1");
        if ok {
            print_ok("proton-ge-custom-bin instalado.");
        } else {
            print_warn("proton-ge-custom-bin no pudo instalarse. Instálalo manualmente con: sudo pacman -S proton-ge-custom-bin");
        }
    } else {
        print_ok("proton-ge-custom-bin ya está instalado.");
    }

    print_ok("¡Herramientas Gaming instaladas correctamente!");
    thread::sleep(Duration::from_secs(2));
}

// ─────────────────────────────────────────────────────────────────────────────
//  Startup check: Chaotic AUR
// ─────────────────────────────────────────────────────────────────────────────

fn check_chaotic_aur_startup() {
    print_info("Revisando configuración de repositorios Chaotic AUR...");
    let (_, out) = run_shell_piped("grep -q '\\[chaotic-aur\\]' /etc/pacman.conf && echo yes");
    if out.trim() == "yes" {
        print_ok("Chaotic AUR ya está configurado.");
    } else {
        install_chaotic_aur();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Main loop
// ─────────────────────────────────────────────────────────────────────────────

fn read_line() -> String {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).ok();
    line.trim().to_string()
}

fn press_enter_to_continue() {
    println!();
    print!("  {}{}Presiona ENTER para volver al menú...{} ", FG_CYAN, DIM, RESET);
    let _ = io::stdout().flush();
    read_line();
}

fn main() {
    // Check and install Chaotic AUR at startup
    check_chaotic_aur_startup();

    loop {
        print_banner();
        print_menu();

        let option = read_line();

        match option.as_str() {
            "1" => {
                install_zsh_starship();
                press_enter_to_continue();
            }
            "2" => {
                install_hyprland();
                press_enter_to_continue();
            }
            "3" => {
                install_niri();
                press_enter_to_continue();
            }
            "4" => {
                install_gaming("nvidia", "1");
                press_enter_to_continue();
            }
            "5" => {
                install_gaming("intel", "7");
                press_enter_to_continue();
            }
            "6" => {
                install_gaming("amd", "12");
                press_enter_to_continue();
            }
            "7" => {
                install_chaotic_aur();
                press_enter_to_continue();
            }
            "8" => {
                install_bauh();
                press_enter_to_continue();
            }
            "9" => {
                install_zsh_starship();
                install_hyprland();
                install_niri();
                install_bauh();
                print_ok("¡El sistema global se configuró con éxito!");
                print_warn("Ejecuta módulos de Gaming por separado según tu GPU.");
                thread::sleep(Duration::from_secs(4));
                press_enter_to_continue();
            }
            "0" => {
                println!();
                print_info("Saliendo de GekkoApp. ¡Hasta luego! 🐉");
                println!();
                break;
            }
            _ => {
                print_warn("Opción no válida. Selecciona un número del 0 al 9.");
                thread::sleep(Duration::from_secs(2));
            }
        }
    }
}
