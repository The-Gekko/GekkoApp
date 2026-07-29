#![allow(dead_code, clippy::print_literal)]
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

// ─────────────────────────────────────────────────────────────────────────────
//  ANSI color / style helpers
// ─────────────────────────────────────────────────────────────────────────────

const RESET: &str = "\x1b[0m";

// Foreground colours
const FG_RED: &str = "\x1b[1;31m";
const FG_GREEN: &str = "\x1b[1;32m";
const FG_YELLOW: &str = "\x1b[1;33m";
const FG_BLUE: &str = "\x1b[1;34m";
const FG_MAGENTA: &str = "\x1b[1;35m";
const FG_CYAN: &str = "\x1b[1;36m";
const FG_WHITE: &str = "\x1b[1;37m";

// Background colours

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

fn hide_cursor() {
    print!("\x1b[?25l");
    let _ = io::stdout().flush();
}
fn show_cursor() {
    print!("\x1b[?25h");
    let _ = io::stdout().flush();
}

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
            FG_MAGENTA, label, RESET, bar, FG_GREEN, BOLD, pct, RESET
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
    let pad_right = width
        .saturating_sub(inner.chars().count())
        .saturating_sub(pad_left);

    println!();
    println!("{}{}╔{}╗{}", FG_MAGENTA, BOLD, "═".repeat(width + 2), RESET);
    println!(
        "{}{}║{}{}{:pad_left$}{}{}{:pad_right$}{}{}║{}",
        FG_MAGENTA,
        BOLD,
        RESET,
        FG_CYAN,
        "",
        inner,
        FG_CYAN,
        "",
        FG_MAGENTA,
        BOLD,
        RESET,
        pad_left = pad_left,
        pad_right = pad_right
    );
    println!("{}{}╚{}╝{}", FG_MAGENTA, BOLD, "═".repeat(width + 2), RESET);
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
    println!("  {}{}╭{}╮{}", FG_CYAN, BOLD, "─".repeat(63), RESET);

    let subtitle = "🐉  THE-GEKKO LINUX POST-INSTALL & CONFIG  🐉";
    let pad = (63usize.saturating_sub(subtitle.chars().count())) / 2;
    println!(
        "  {}{}│{}{}{}{}{}{:>pad$}{}│{}",
        FG_CYAN,
        BOLD,
        RESET,
        " ".repeat(pad),
        FG_WHITE,
        BOLD,
        subtitle,
        "",
        FG_CYAN,
        RESET,
        pad = 0
    );

    let by_line = "Arch Linux ❱ Automated Setup ❱ Powered by Rust";
    println!(
        "  {}{}|  {}{}{}{}",
        FG_CYAN, BOLD, DIM, FG_CYAN, by_line, RESET
    );

    println!("  {}{}╰{}╯{}", FG_CYAN, BOLD, "─".repeat(63), RESET);

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
        MenuItem {
            key: "1",
            icon: "🐚",
            label: "Terminal Bonita           (ZSH + Starship + Plugins)",
            badge: None,
        },
        MenuItem {
            key: "2",
            icon: "🪟",
            label: "Entorno Hyprland          (Herramientas + Deps)",
            badge: None,
        },
        MenuItem {
            key: "3",
            icon: "🪟",
            label: "Entorno Niri              (Herramientas + Deps)",
            badge: None,
        },
        MenuItem {
            key: "4",
            icon: "🎮",
            label: "Gaming Setup",
            badge: Some(("NVIDIA", FG_GREEN)),
        },
        MenuItem {
            key: "5",
            icon: "🎮",
            label: "Gaming Setup",
            badge: Some(("INTEL", FG_BLUE)),
        },
        MenuItem {
            key: "6",
            icon: "🎮",
            label: "Gaming Setup",
            badge: Some(("AMD", FG_RED)),
        },
        MenuItem {
            key: "7",
            icon: "📦",
            label: "Agregar repositorios      Chaotic AUR",
            badge: None,
        },
        MenuItem {
            key: "8",
            icon: "🛍️",
            label: "Tienda Bauh               (Parcheado + AUR)",
            badge: None,
        },
        MenuItem {
            key: "9",
            icon: "✨",
            label: "Instalar TODO",
            badge: Some(("Terminal+Hyprland+Niri+Bauh", FG_YELLOW)),
        },
        MenuItem {
            key: "0",
            icon: "❌",
            label: "Salir",
            badge: None,
        },
    ];

    println!(
        "  {}{}Selecciona una opción para configurar tu entorno:{}",
        FG_WHITE, BOLD, RESET
    );
    println!();

    for item in &items {
        let key_color = if item.key == "0" { FG_RED } else { FG_CYAN };
        print!(
            "  {}{}[{}]{} {} {}{}{}",
            key_color, BOLD, item.key, RESET, item.icon, FG_WHITE, item.label, RESET
        );
        if let Some((badge_text, badge_color)) = item.badge {
            print!("  {}{}{}{}", badge_color, BOLD, badge_text, RESET);
        }
        println!();
    }

    println!();
    println!(
        "  {}{}{}─────────────────────────────────────────────────────────────{}",
        FG_CYAN, BOLD, DIM, RESET
    );
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
    let out = Command::new("bash").arg("-c").arg(cmd).output();
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

fn check_arch_linux() -> bool {
    Path::new("/etc/arch-release").exists()
}

fn confirm_action(prompt: &str) -> bool {
    print!("  {}  ❓  {} [s/N]:{} ", FG_YELLOW, prompt, RESET);
    let _ = io::stdout().flush();
    let ans = read_line().to_lowercase();
    ans == "s" || ans == "y"
}

fn read_line() -> String {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).ok();
    line.trim().to_string()
}

fn press_enter_to_continue() {
    println!();
    print!(
        "  {}{}Presiona ENTER para volver al menú...{} ",
        FG_CYAN, DIM, RESET
    );
    let _ = io::stdout().flush();
    read_line();
}

fn instalar_paquetes(paquetes: &[&str]) -> bool {
    if !check_arch_linux() {
        print_err("El sistema no es Arch Linux. No se pueden instalar paquetes con pacman.");
        return false;
    }

    let faltantes: Vec<&str> = paquetes
        .iter()
        .filter(|&&p| !is_package_installed(p))
        .copied()
        .collect();

    if faltantes.is_empty() {
        print_ok("Todos los paquetes ya están instalados. Saltando...");
        return true;
    }

    println_colored(
        &format!("📦  Se instalarán {} paquetes faltantes:", faltantes.len()),
        FG_YELLOW,
    );
    for pkg in &faltantes {
        print_step(&format!("→ {}", pkg));
    }

    if !confirm_action("¿Deseas continuar con la instalación de estos paquetes?") {
        print_warn("Instalación de paquetes cancelada por el usuario.");
        return false;
    }

    let pkg_list = faltantes.join(" ");
    let cmd = format!("sudo pacman -S --needed --noconfirm {}", pkg_list);
    fake_progress_bar("Instalando", 40);
    if !run_shell(&cmd) {
        print_err("Algunos paquetes no pudieron instalarse. Revisa la salida anterior.");
        return false;
    }
    true
}

fn desinstalar_paquetes(paquetes: &[&str]) -> bool {
    if !check_arch_linux() {
        return false;
    }

    let a_eliminar: Vec<&str> = paquetes
        .iter()
        .filter(|&&p| is_package_installed(p))
        .copied()
        .collect();

    if a_eliminar.is_empty() {
        print_ok("Esos paquetes ya no están en el sistema. Saltando desinstalación...");
        return true;
    }

    println_colored(
        &format!(
            "🗑️  Se van a desinstalar {} paquetes innecesarios:",
            a_eliminar.len()
        ),
        FG_RED,
    );
    for pkg in &a_eliminar {
        print_step(&format!("✗ {}", pkg));
    }

    if !confirm_action("¿Deseas proceder con la desinstalación de estos paquetes?") {
        print_warn("Desinstalación cancelada por el usuario.");
        return false;
    }

    let pkg_list = a_eliminar.join(" ");
    let cmd = format!("sudo pacman -Rns --noconfirm {}", pkg_list);
    if !run_shell(&cmd) {
        print_err("Error al desinstalar paquetes.");
        return false;
    }
    true
}

// ─────────────────────────────────────────────────────────────────────────────
//  Fastfetch config
// ─────────────────────────────────────────────────────────────────────────────

fn configurar_fastfetch() -> bool {
    print_info("Aplicando el tema universal de Fastfetch...");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let config_dir = format!("{}/.config/fastfetch", home);
    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        print_err(&format!(
            "No se pudo crear el directorio de fastfetch: {}",
            e
        ));
        return false;
    }

    let config = format!(
        r#"{{
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
"#,
        home = home
    );

    let config_path = format!("{}/config.jsonc", config_dir);
    if Path::new(&config_path).exists() {
        print_info(&format!("Se sobrescribirá el archivo {}", config_path));
        if !confirm_action("¿Deseas sobrescribir config.jsonc de fastfetch?") {
            return false;
        }
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if let Err(e) = std::fs::copy(
            &config_path,
            format!("{}.backup_{}", config_path, timestamp),
        ) {
            print_err(&format!("Fallo al respaldar config.jsonc: {}", e));
            return false;
        }
    }

    if let Err(e) = std::fs::write(&config_path, config) {
        print_err(&format!("Fallo al escribir config.jsonc: {}", e));
        return false;
    }
    print_ok("Configuración .jsonc de Fastfetch generada.");

    // Copy image if available
    let img_dest = format!("{}/Anime Render.png", config_dir);
    if !Path::new(&img_dest).exists() {
        if Path::new("./Anime Render.png").exists() {
            if let Err(e) = std::fs::copy("./Anime Render.png", &img_dest) {
                print_warn(&format!("No se pudo copiar la imagen: {}", e));
            } else {
                print_ok("Imagen 'Anime Render.png' copiada a ~/.config/fastfetch/");
            }
        } else {
            print_warn(
                "Copia tu imagen 'Anime Render.png' en ~/.config/fastfetch/ para el logo kitty.",
            );
        }
    }

    true
}

// ─────────────────────────────────────────────────────────────────────────────
//  Install functions
// ─────────────────────────────────────────────────────────────────────────────

fn install_zsh_starship() -> bool {
    print_header("INSTALANDO ZSH Y TERMINAL BONITA  (by GekkoApp)");
    thread::sleep(Duration::from_millis(500));

    if !instalar_paquetes(&[
        "kitty",
        "git",
        "zsh",
        "nano",
        "curl",
        "eza",
        "fastfetch",
        "fzf",
        "ttf-jetbrains-mono-nerd",
        "starship",
        "zsh-autosuggestions",
        "zsh-syntax-highlighting",
        "zsh-history-substring-search",
        "zsh-completions",
        "zoxide",
    ]) {
        print_err("Fallo la instalación de dependencias base y plugins de ZSH.");
        return false;
    }

    if !run_shell("fc-cache -fv > /dev/null 2>&1") {
        print_warn("Fallo al actualizar caché de fuentes (fc-cache).");
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
fpath=(/usr/share/zsh/plugins/zsh-completions/src $fpath)
compinit -d ~/.zcompdump

zstyle ':completion:*' matcher-list 'm:{a-z}={A-Za-z}' 'r:|[._-]=* r:|=*'

setopt AUTO_MENU
setopt COMPLETE_IN_WORD
setopt ALWAYS_TO_END

# =====================================
# ALIASES
# =====================================

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

source /usr/share/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh
source /usr/share/zsh/plugins/zsh-history-substring-search/zsh-history-substring-search.zsh
bindkey '^[[A' history-substring-search-up
bindkey '^[[B' history-substring-search-down
source /usr/share/fzf/key-bindings.zsh
source /usr/share/fzf/completion.zsh
source /usr/share/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh
eval "$(zoxide init zsh)"

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
    let zshrc_path = format!("{}/.zshrc", home);

    print_info(&format!("Se sobrescribirá el archivo {}", zshrc_path));
    if confirm_action("¿Deseas sobrescribir ~/.zshrc con la nueva configuración?") {
        if Path::new(&zshrc_path).exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if let Err(e) =
                std::fs::copy(&zshrc_path, format!("{}.backup_{}", zshrc_path, timestamp))
            {
                print_err(&format!("Fallo al respaldar .zshrc: {}", e));
                return false;
            }
        }
        if let Err(e) = std::fs::write(&zshrc_path, zshrc) {
            print_err(&format!("Fallo al escribir .zshrc: {}", e));
            return false;
        }
        print_ok(".zshrc escrito correctamente.");
    }

    if confirm_action("¿Deseas aplicar el preset 'jetpack' de Starship?") {
        print_info("Estableciendo preset de Starship...");
        let _ = run_shell("mkdir -p ~/.config");
        let starship_cfg = format!("{}/.config/starship.toml", home);
        if Path::new(&starship_cfg).exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if let Err(e) = std::fs::copy(
                &starship_cfg,
                format!("{}.backup_{}", starship_cfg, timestamp),
            ) {
                print_err(&format!("Fallo al respaldar starship.toml: {}", e));
                return false;
            }
        }
        if !run_shell("starship preset jetpack -o ~/.config/starship.toml") {
            print_err("Fallo al escribir el preset de Starship.");
            return false;
        }
    }

    if !configurar_fastfetch() {
        print_err("Fallo la configuración de fastfetch.");
        return false;
    }

    if confirm_action("¿Deseas cambiar tu shell predeterminada a ZSH?")
        && !run_shell("sudo chsh -s /bin/zsh $USER")
    {
        print_err("Fallo al cambiar la shell.");
        return false;
    }

    print_ok("¡Instalación de Terminal Finalizada! — by GekkoApp");
    print_warn("Reinicia sesión o ejecuta: exec zsh  para aplicar cambios.");
    thread::sleep(Duration::from_secs(2));
    true
}

fn install_hyprland() -> bool {
    print_header("INSTALANDO PRESET HYPRLAND");
    if !instalar_paquetes(&[
        "gedit",
        "electron",
        "gnome-keyring",
        "polkit-gnome",
        "xdg-desktop-portal-gtk",
        "xwayland-satellite",
        "nwg-look",
        "blueman",
        "ibus",
        "unzip",
        "colord",
        "bolt",
        "flatpak",
        "gnome-disk-utility",
        "gvfs-mtp",
        "gvfs-gphoto2",
        "libmtp",
        "nautilus",
        "fuse2",
        "ddcutil",
        "i2c-dev",
    ]) {
        return false;
    }

    if !desinstalar_paquetes(&["dolphin", "polkit-kde-agent", "wofi"]) {
        print_warn("Hubo un problema o se canceló la desinstalación. Abortando instalación.");
        return false;
    }
    print_ok("Evaluación de dependencias de Hyprland finalizada.");
    thread::sleep(Duration::from_secs(2));
    true
}

fn install_niri() -> bool {
    print_header("INSTALANDO PRESET NIRI");
    if !instalar_paquetes(&[
        "gedit",
        "xdg-desktop-portal-gnome",
        "gnome-keyring",
        "polkit-gnome",
        "xdg-desktop-portal-gtk",
        "electron",
        "xwayland-satellite",
        "gnome-disk-utility",
        "qt5-wayland",
        "qt6-wayland",
        "nwg-look",
        "flatpak",
        "blueman",
        "gvfs-mtp",
        "gvfs-gphoto2",
        "libmtp",
        "ibus",
        "unzip",
        "colord",
        "bolt",
        "fuse2",
        "ddcutil",
        "i2c-dev",
        "playerctl",
        "gpsd",
        "dconf-editor",
    ]) {
        return false;
    }

    if !desinstalar_paquetes(&["mako", "swaybg", "swayidle", "swaylock", "waybar"]) {
        print_warn("Hubo un problema o se canceló la desinstalación. Abortando instalación.");
        return false;
    }
    print_ok("Evaluación de dependencias de Niri finalizada.");
    thread::sleep(Duration::from_secs(2));
    true
}

fn check_chaotic_aur_configured(pacman_conf: &str) -> bool {
    let mut in_chaotic = false;
    let mut has_siglevel = false;
    let mut has_include = false;

    for line in pacman_conf.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if trimmed == "[chaotic-aur]" {
                in_chaotic = true;
                has_siglevel = false;
                has_include = false;
            } else if in_chaotic {
                if has_siglevel && has_include {
                    return true;
                }
                in_chaotic = false;
                has_siglevel = false;
                has_include = false;
            }
        } else if in_chaotic {
            if let Some((key, val)) = trimmed.split_once('=') {
                let key = key.trim();
                let val = val.trim();
                if key == "SigLevel" && val == "Required DatabaseOptional" {
                    has_siglevel = true;
                } else if key == "Include" && val == "/etc/pacman.d/chaotic-mirrorlist" {
                    has_include = true;
                }
            }
        }
    }

    in_chaotic && has_siglevel && has_include
}

enum ReplaceResult {
    ConcurrentModification,
    NotReplaced,
    Replaced,
}

struct LockGuard<'a> {
    path: &'a str,
}

impl<'a> Drop for LockGuard<'a> {
    fn drop(&mut self) {
        let _ = std::process::Command::new("sudo")
            .args(["rmdir", self.path])
            .status();
    }
}

fn replace_pacman_conf_securely(current_conf: &str, new_conf: &str) -> ReplaceResult {
    let mktemp_cmd = std::process::Command::new("sudo")
        .arg("mktemp")
        .arg("/etc/pacman.conf.tmp.XXXXXX")
        .output();

    let tmp_file = match mktemp_cmd {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => return ReplaceResult::NotReplaced,
    };

    let cleanup = |file: &str| {
        let _ = std::process::Command::new("sudo")
            .arg("rm")
            .arg("-f")
            .arg(file)
            .status();
    };

    let mut tee = match std::process::Command::new("sudo")
        .arg("tee")
        .arg(&tmp_file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    };

    if let Some(mut stdin) = tee.stdin.take() {
        if std::io::Write::write_all(&mut stdin, new_conf.as_bytes()).is_err() {
            let _ = tee.wait();
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    }

    match tee.wait() {
        Ok(status) if status.success() => {}
        _ => {
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    }

    match std::process::Command::new("sudo")
        .args(["chmod", "644", &tmp_file])
        .status()
    {
        Ok(status) if status.success() => {}
        _ => {
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    }
    match std::process::Command::new("sudo")
        .args(["chown", "root:root", &tmp_file])
        .status()
    {
        Ok(status) if status.success() => {}
        _ => {
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    }

    if !check_chaotic_aur_configured(new_conf) {
        cleanup(&tmp_file);
        return ReplaceResult::NotReplaced;
    }

    let active_conf = match std::fs::read_to_string("/etc/pacman.conf") {
        Ok(c) => c,
        Err(_) => {
            cleanup(&tmp_file);
            return ReplaceResult::NotReplaced;
        }
    };
    if active_conf != current_conf {
        cleanup(&tmp_file);
        return ReplaceResult::ConcurrentModification;
    }

    let mv_status = std::process::Command::new("sudo")
        .args(["mv", &tmp_file, "/etc/pacman.conf"])
        .status();

    if let Ok(st) = mv_status {
        if st.success() {
            ReplaceResult::Replaced
        } else {
            cleanup(&tmp_file);
            ReplaceResult::NotReplaced
        }
    } else {
        cleanup(&tmp_file);
        ReplaceResult::NotReplaced
    }
}

fn install_chaotic_aur() -> bool {
    print_header("AGREGANDO REPOSITORIOS CHAOTIC AUR");

    if !check_arch_linux() {
        print_err("El sistema no es Arch Linux. No se pueden configurar los repositorios.");
        return false;
    }

    if !confirm_action("¿Deseas instalar y configurar Chaotic AUR?") {
        return false;
    }

    let lock_path = "/run/lock/gekkoapp_pacman_conf_lock";
    if !run_shell(&format!("sudo mkdir {}", lock_path)) {
        print_err("No se pudo obtener el bloqueo de transacción. ¿Otra instancia en ejecución?");
        return false;
    }
    let _lock = LockGuard { path: lock_path };

    let keyring_installed = run_shell("pacman -Qq chaotic-keyring >/dev/null 2>&1");
    let mirrorlist_installed = run_shell("pacman -Qq chaotic-mirrorlist >/dev/null 2>&1");

    let pacman_conf = match std::fs::read_to_string("/etc/pacman.conf") {
        Ok(conf) => conf,
        Err(_) => {
            print_err("Error crítico: No se pudo leer /etc/pacman.conf");
            return false;
        }
    };

    let repo_configured = check_chaotic_aur_configured(&pacman_conf);

    if keyring_installed && mirrorlist_installed && repo_configured {
        print_ok("Chaotic AUR ya está completamente configurado.");
        thread::sleep(Duration::from_secs(2));
        return true;
    }

    if !keyring_installed || !mirrorlist_installed {
        print_info("Obteniendo y verificando llaves públicas...");
        let key_success = run_shell("sudo pacman-key --recv-key 3056513887B78AEB --keyserver hkps://keys.openpgp.org || sudo pacman-key --recv-key 3056513887B78AEB --keyserver keyserver.ubuntu.com");
        if !key_success {
            print_err("Error crítico: No se pudo obtener la llave GPG de Chaotic AUR.");
            return false;
        }

        if !run_shell("sudo pacman-key --lsign-key 3056513887B78AEB") {
            print_err("Error al firmar la llave GPG.");
            return false;
        }

        print_info("Instalando keyring y mirrorlist de Chaotic AUR...");
        if !run_shell("sudo pacman -U --noconfirm 'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst' 'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst'") {
            print_err("Error instalando paquetes de Chaotic AUR.");
            return false;
        }
    }

    let current_conf = match std::fs::read_to_string("/etc/pacman.conf") {
        Ok(conf) => conf,
        Err(_) => {
            print_err("Error crítico: No se pudo releer /etc/pacman.conf");
            return false;
        }
    };

    if !check_chaotic_aur_configured(&current_conf) {
        print_info("Editando /etc/pacman.conf de manera segura...");
        let (ok, out) = run_shell_piped("sudo mktemp /etc/pacman.conf.backup.XXXXXX");
        let backup_file = if ok {
            out.trim().to_string()
        } else {
            print_err("No se pudo generar nombre seguro para el backup.");
            return false;
        };

        let mut tee = match std::process::Command::new("sudo")
            .arg("tee")
            .arg(&backup_file)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => return false,
        };
        if let Some(mut stdin) = tee.stdin.take() {
            if std::io::Write::write_all(&mut stdin, current_conf.as_bytes()).is_err() {
                let _ = tee.wait();
                return false;
            }
        }
        if !tee.wait().map(|s| s.success()).unwrap_or(false) {
            print_err("No se pudo respaldar pacman.conf.");
            return false;
        }

        let mut new_conf = String::new();
        let mut in_chaotic = false;
        for line in current_conf.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("[chaotic-aur]") {
                in_chaotic = true;
                continue;
            }
            if in_chaotic && trimmed.starts_with('[') {
                in_chaotic = false;
            }
            if !in_chaotic {
                new_conf.push_str(line);
                new_conf.push('\n');
            }
        }
        new_conf.push_str("\n[chaotic-aur]\nSigLevel = Required DatabaseOptional\nInclude = /etc/pacman.d/chaotic-mirrorlist\n");

        match replace_pacman_conf_securely(&current_conf, &new_conf) {
            ReplaceResult::ConcurrentModification => {
                print_err("Archivo pacman.conf modificado por otro proceso. Abortando.");
                return false;
            }
            ReplaceResult::NotReplaced => {
                print_err("Fallo al preparar o reemplazar pacman.conf.");
                return false;
            }
            ReplaceResult::Replaced => {
                print_info("Sincronizando repositorios y actualizando el sistema...");
                if !run_shell("sudo pacman -Syu") {
                    print_err("Error sincronizando repositorios. Restaurando backup...");
                    match std::fs::read_to_string("/etc/pacman.conf") {
                        Ok(active) => {
                            if active == new_conf {
                                let restore_cmd =
                                    format!("sudo mv {} /etc/pacman.conf", backup_file);
                                if !run_shell(&restore_cmd) {
                                    print_err("ERROR CRÍTICO: No se pudo restaurar pacman.conf.");
                                }
                            } else {
                                print_warn(
                                    "pacman.conf modificado tras nuestra escritura. Backup descartado.",
                                );
                            }
                        }
                        Err(e) => {
                            print_err(&format!(
                                "Error crítico leyendo pacman.conf en rollback: {}",
                                e
                            ));
                            print_err(&format!(
                                "El respaldo manual está disponible en: {}",
                                backup_file
                            ));
                        }
                    }
                    return false;
                }
            }
        }
    } else {
        print_info("Sincronizando repositorios y actualizando el sistema...");
        if !run_shell("sudo pacman -Syu") {
            print_err("Error sincronizando repositorios (pacman -Syu falló).");
            return false;
        }
    }

    print_ok("¡Repositorios Chaotic AUR configurados con éxito!");
    thread::sleep(Duration::from_secs(2));
    true
}

fn install_bauh() -> bool {
    print_header("INSTALANDO TIENDA BAUH (AUR)");
    if !instalar_paquetes(&["bauh"]) {
        return false;
    }

    print_info("Buscando archivo gems.py de bauh...");
    let (ok, gems_path) = run_shell_piped(
        "ls /usr/lib/python*/site-packages/bauh/view/core/gems.py 2>/dev/null | head -n 1",
    );

    let gems_path = gems_path.trim().to_string();
    if !ok || gems_path.is_empty() {
        print_warn("No se encontró el archivo gems.py de bauh. Omitiendo modificaciones.");
        thread::sleep(Duration::from_secs(2));
        return true;
    }

    print_info("No se parcheará gems.py directamente para evitar romper paquetes de pacman.");
    print_info("Bauh ha sido instalado exitosamente.");

    thread::sleep(Duration::from_secs(2));
    true
}

fn install_gaming(gpu: &str, vulkan_choice: &str) -> bool {
    let label = match gpu {
        "nvidia" => "INSTALANDO UTILIDADES GAMING  ·  NVIDIA",
        "intel" => "INSTALANDO UTILIDADES GAMING  ·  INTEL",
        "amd" => "INSTALANDO UTILIDADES GAMING  ·  AMD",
        _ => "INSTALANDO UTILIDADES GAMING",
    };
    print_header(label);

    if !check_arch_linux() {
        print_err("El sistema no es Arch Linux. Cancelando.");
        return false;
    }

    if !confirm_action(&format!(
        "¿Deseas proceder con la instalación del entorno Gaming para {}?",
        gpu.to_uppercase()
    )) {
        return false;
    }

    // Eliminado pacman -Sy aislado según reglas de Arch Linux sobre actualizaciones parciales.

    // Steam: pipe the vulkan driver selection answer
    print_info("Instalando steam...");
    let steam_cmd = format!(
        "echo -e '{}\\ns' | sudo pacman -S --needed steam",
        vulkan_choice
    );
    if !run_shell(&steam_cmd) {
        print_warn("Steam no se pudo instalar correctamente o fue cancelado.");
        return false;
    }

    print_info("Instalando dxvk (seleccionando proveedor 1 automáticamente)...");
    if !is_package_installed("dxvk-async-git") && !is_package_installed("dxvk-mingw-git") {
        if !run_shell("echo '1' | sudo pacman -S --needed dxvk 2>&1") {
            print_warn("Fallo al instalar dxvk.");
            return false;
        }
    } else {
        print_ok("dxvk ya está instalado. Saltando...");
    }

    print_info("Instalando dependencias de Proton GE (lib32-gstreamer)...");
    if !instalar_paquetes(&["lib32-gstreamer", "lib32-gst-plugins-base"]) {
        print_warn("Fallo al instalar dependencias de lib32.");
        return false;
    }

    if !instalar_paquetes(&[
        "protonplus",
        "spotify",
        "discord",
        "gamemode",
        "gedit",
        "flatpak",
    ]) {
        print_warn("Fallo al instalar algunas herramientas gaming.");
        return false;
    }

    print_info("Instalando proton-ge-custom-bin...");
    if !is_package_installed("proton-ge-custom-bin") {
        let ok = run_shell("sudo pacman -S --needed --noconfirm proton-ge-custom-bin 2>&1");
        if ok {
            print_ok("proton-ge-custom-bin instalado.");
        } else {
            print_warn("proton-ge-custom-bin no pudo instalarse. Instálalo manualmente con: sudo pacman -S proton-ge-custom-bin");
            return false;
        }
    } else {
        print_ok("proton-ge-custom-bin ya está instalado.");
    }

    print_ok("¡Proceso de instalación Gaming terminado!");
    thread::sleep(Duration::from_secs(2));
    true
}

// ─────────────────────────────────────────────────────────────────────────────
//  Startup check: Diagnostic Only
// ─────────────────────────────────────────────────────────────────────────────

fn check_chaotic_aur_startup() {
    print_info("Ejecutando diagnóstico inicial del sistema...");

    if check_arch_linux() {
        print_ok("Sistema operativo detectado: Arch Linux");

        let pacman_conf = std::fs::read_to_string("/etc/pacman.conf").unwrap_or_default();
        if check_chaotic_aur_configured(&pacman_conf) {
            print_ok("Repositorio Chaotic AUR: CONFIGURADO");
        } else {
            print_warn("Repositorio Chaotic AUR: NO CONFIGURADO");
        }
    } else {
        print_warn(
            "Sistema operativo detectado: NO es Arch Linux. Algunas funciones estarán limitadas.",
        );
    }

    thread::sleep(Duration::from_secs(2));
}

// ─────────────────────────────────────────────────────────────────────────────
//  Main loop
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    // Check Chaotic AUR at startup as diagnostic only
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
                if !confirm_action("¿Deseas iniciar la instalación completa del entorno?") {
                    continue;
                }

                let eleccion_global;
                loop {
                    println!();
                    println!("¿Qué entorno gráfico deseas instalar?");
                    println!("1) Hyprland");
                    println!("2) Niri");
                    println!("3) Cancelar operación global");
                    print!("Elige (1/2/3): ");
                    let _ = io::stdout().flush();
                    let eleccion = read_line();

                    if eleccion == "1" || eleccion == "2" {
                        eleccion_global = eleccion;
                        break;
                    } else if eleccion == "3" {
                        eleccion_global = eleccion;
                        print_warn("Operación cancelada por el usuario.");
                        break;
                    } else {
                        print_warn("Opción no válida. Por favor, elige 1, 2 o 3.");
                    }
                }

                if eleccion_global == "3" {
                    press_enter_to_continue();
                    continue;
                }

                if !install_zsh_starship() {
                    print_err("La instalación de ZSH falló. Abortando 'Instalar TODO'.");
                    press_enter_to_continue();
                    continue;
                }

                let ok = if eleccion_global == "1" {
                    install_hyprland()
                } else {
                    install_niri()
                };

                if !ok {
                    print_err(
                        "La instalación del entorno gráfico falló o fue cancelada. Abortando 'Instalar TODO'.",
                    );
                    press_enter_to_continue();
                    continue;
                }

                if !install_bauh() {
                    print_err("La instalación de Bauh falló. Abortando 'Instalar TODO'.");
                    press_enter_to_continue();
                    continue;
                }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_chaotic_aur_configured() {
        let valid = "
[options]
Architecture = auto

[chaotic-aur]
SigLevel = Required DatabaseOptional
Include = /etc/pacman.d/chaotic-mirrorlist
        ";
        assert!(check_chaotic_aur_configured(valid));

        let incomplete = "
[chaotic-aur]
SigLevel = Required DatabaseOptional
        ";
        assert!(!check_chaotic_aur_configured(incomplete));

        let wrong_keys = "
[chaotic-aur]
NotSigLevel = Required DatabaseOptional
BadInclude = /etc/pacman.d/chaotic-mirrorlist
        ";
        assert!(!check_chaotic_aur_configured(wrong_keys));

        let multiple_blocks_no_merge = "
[chaotic-aur]
SigLevel = Required DatabaseOptional

[chaotic-aur]
Include = /etc/pacman.d/chaotic-mirrorlist
        ";
        assert!(!check_chaotic_aur_configured(multiple_blocks_no_merge));

        let commented = "
#[chaotic-aur]
#SigLevel = Required DatabaseOptional
#Include = /etc/pacman.d/chaotic-mirrorlist
        ";
        assert!(!check_chaotic_aur_configured(commented));

        let mixed_repos = "
[core]
Include = /etc/pacman.d/mirrorlist

[chaotic-aur]
SigLevel = Required DatabaseOptional
Include = /etc/pacman.d/chaotic-mirrorlist

[extra]
Include = /etc/pacman.d/mirrorlist
        ";
        assert!(check_chaotic_aur_configured(mixed_repos));
    }
}
