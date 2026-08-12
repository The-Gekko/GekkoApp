#![allow(dead_code, clippy::print_literal)]

use gekkoapp::core::flow::{
    install_bauh, install_gaming, install_gekko_adb, install_gekkoapp, install_hyprland,
    install_kito_environment, install_niri, install_zsh_starship,
};
use gekkoapp::core::pacman::install_chaotic_aur;
use gekkoapp::core::reporter::{BOLD, DIM, FG_CYAN, FG_GREEN, FG_MAGENTA, FG_RED, FG_WHITE, RESET};
use gekkoapp::core::{CliReporter, Reporter};
use gekkoapp::environment::SystemEnvironment;
use std::io::{self, BufRead, Write};

fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

fn read_line() -> String {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).ok();
    line.trim().to_string()
}

fn press_enter_to_continue(reporter: &dyn Reporter) {
    println!();
    print!(
        "  {}{}Presiona ENTER para volver al menú...{} ",
        FG_CYAN, DIM, RESET
    );
    let _ = io::stdout().flush();
    let _ = reporter.read_line();
}

// ─────────────────────────────────────────────────────────────────────────────
//  ASCII art banner
// ─────────────────────────────────────────────────────────────────────────────

fn print_banner() {
    clear_screen();

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
    for clr in bar_colors.iter().cycle().take(65) {
        print!("{}▀", clr);
    }
    println!("{}", RESET);

    println!();

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
    badge: Option<(&'static str, &'static str)>,
}

fn print_menu() {
    let items = [
        MenuItem {
            key: "K",
            icon: "🦊",
            label: "Instalar entorno Kito     (KiUI + modulos)",
            badge: Some(("NUEVO", FG_MAGENTA)),
        },
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
            badge: Some(("INTEL", FG_CYAN)),
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
            key: "b",
            icon: "📱",
            label: "Gekko ADB Studio         (Control ADB GTK)",
            badge: Some(("NUEVO", FG_MAGENTA)),
        },
        MenuItem {
            key: "u",
            icon: "🔄",
            label: "Actualizar GekkoApp        (Auto-update)",
            badge: Some(("NUEVO", FG_MAGENTA)),
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

fn main() {
    let reporter = CliReporter;
    let environment = SystemEnvironment::detect();
    loop {
        print_banner();
        print_menu();

        let option = read_line();

        match option.as_str() {
            "k" | "K" => {
                install_kito_environment(&reporter);
                press_enter_to_continue(&reporter);
            }
            "1" => {
                install_zsh_starship(&reporter);
                press_enter_to_continue(&reporter);
            }
            "2" => {
                install_hyprland(&reporter);
                press_enter_to_continue(&reporter);
            }
            "3" => {
                install_niri(&reporter);
                press_enter_to_continue(&reporter);
            }
            "4" => {
                install_gaming(&reporter, "nvidia", "1");
                press_enter_to_continue(&reporter);
            }
            "5" => {
                install_gaming(&reporter, "intel", "7");
                press_enter_to_continue(&reporter);
            }
            "6" => {
                install_gaming(&reporter, "amd", "12");
                press_enter_to_continue(&reporter);
            }
            "7" => {
                install_chaotic_aur(&reporter);
                press_enter_to_continue(&reporter);
            }
            "8" => {
                match install_bauh(&reporter, &environment, true) {
                    Ok(()) => {}
                    Err(error) => reporter.err(&format!("Bauh Fork no se instalo: {error}")),
                }
                press_enter_to_continue(&reporter);
            }
            "b" | "B" => {
                match install_gekko_adb(&reporter) {
                    Ok(()) => {}
                    Err(error) => reporter.err(&format!("Gekko ADB Studio no se instalo: {error}")),
                }
                press_enter_to_continue(&reporter);
            }
            "u" | "U" => {
                match install_gekkoapp(&reporter, &environment, true) {
                    Ok(()) => {}
                    Err(error) => reporter.err(&format!("GekkoApp no se actualizó: {error}")),
                }
                press_enter_to_continue(&reporter);
            }
            "0" => {
                println!();
                reporter.info("Saliendo de GekkoApp. ¡Hasta luego! 🐉");
                println!();
                break;
            }
            _ => {
                reporter.warn("Opción no válida. Selecciona una opción del menú.");
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    }
}
