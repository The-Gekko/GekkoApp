#![allow(dead_code, clippy::print_literal)]

use gekkoapp::core::flow::{
    install_bauh, install_gekko_adb, install_gekkoapp, install_kito_environment, uninstall_bauh,
    uninstall_gekko_adb, uninstall_kito_environment,
};
use gekkoapp::core::pacman::install_chaotic_aur;

use gekkoapp::core::reporter::{BOLD, DIM, FG_CYAN, FG_MAGENTA, FG_RED, FG_WHITE, RESET};
use gekkoapp::core::{CliReporter, Reporter};
use gekkoapp::environment::SystemEnvironment;
use std::io::{self, BufRead, Write};

fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

/// Lee una linea del menu. Devuelve `None` cuando la entrada se ha cerrado
/// (EOF: `gekkoapp </dev/null`, una tuberia agotada, Ctrl+D): antes se
/// trataba como una opcion vacia y el menu se redibujaba en un bucle infinito.
fn read_line() -> Option<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line.trim().to_string()),
    }
}

fn print_usage() {
    println!(
        "GekkoApp {} - menu de post-instalacion (CLI)",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Uso: gekkoapp [--help | --version]");
    println!();
    println!("Sin argumentos abre el menu interactivo (instalar, actualizar y");
    println!("desinstalar Kito, Bauh Fork y Gekko ADB Studio, y agregar Chaotic AUR).");
    println!("Con la entrada cerrada (EOF) el menu termina en vez de repetirse.");
    println!();
    println!("  -h, --help      Muestra esta ayuda y sale.");
    println!("  -V, --version   Muestra la version y sale.");
    println!();
    println!("El Control Center grafico es el binario gekkoapp-gui.");
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

fn print_banner(env: &SystemEnvironment) {
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
    let width = 63usize;
    let left = width.saturating_sub(subtitle.chars().count()) / 2;
    let right = width
        .saturating_sub(subtitle.chars().count())
        .saturating_sub(left);
    println!(
        "  {}{}│{}{}{}{}{}{}{}{}│{}",
        FG_CYAN,
        BOLD,
        RESET,
        " ".repeat(left),
        FG_WHITE,
        BOLD,
        subtitle,
        RESET,
        " ".repeat(right),
        FG_CYAN,
        RESET
    );

    let by_line = format!(
        "{} ❱ Automated Setup ({}) ❱ Powered by Rust",
        env.distro_name, env.package_manager
    );
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

fn print_menu(env: &SystemEnvironment) {
    let is_arch = env.distro_id == "arch" || env.distro_like.iter().any(|id| id == "arch");
    let is_solus = env.distro_id == "solus" || env.distro_like.iter().any(|id| id == "solus");

    let mut items = Vec::new();

    if env.compatibility.supported {
        items.push(MenuItem {
            key: "K",
            icon: "🦊",
            label: "Instalar entorno Kito     (KiUI + modulos)",
            badge: Some(("NUEVO", FG_MAGENTA)),
        });
    }

    if is_arch {
        items.push(MenuItem {
            key: "1",
            icon: "📦",
            label: "Agregar repositorios      Chaotic AUR",
            badge: None,
        });
    }

    if is_arch || is_solus {
        items.push(MenuItem {
            key: "2",
            icon: "🛍️",
            label: "Tienda Bauh               (Parcheado + AUR)",
            badge: None,
        });
        items.push(MenuItem {
            key: "3",
            icon: "📱",
            label: "Gekko ADB Studio         (Control ADB GTK)",
            badge: Some(("NUEVO", FG_MAGENTA)),
        });
    }

    items.push(MenuItem {
        key: "u",
        icon: "🔄",
        label: "Actualizar GekkoApp        (Auto-update)",
        badge: Some(("NUEVO", FG_MAGENTA)),
    });

    items.push(MenuItem {
        key: "d",
        icon: "🗑️ ",
        label: "Desinstalar componentes   (Submenú de desinstalación)",
        badge: Some(("DESINSTALADOR", FG_RED)),
    });

    items.push(MenuItem {
        key: "0",
        icon: "❌",
        label: "Salir",
        badge: None,
    });

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

fn run_uninstall_menu(reporter: &dyn Reporter) {
    println!();
    println!(
        "  {}{}--- MENÚ DE DESINSTALACIÓN ---{}",
        FG_WHITE, BOLD, RESET
    );
    println!("  {}[1]{} Desinstalar Tienda Bauh Fork", FG_CYAN, RESET);
    println!("  {}[2]{} Desinstalar Gekko ADB Studio", FG_CYAN, RESET);
    println!("  {}[3]{} Desinstalar Entorno Kito", FG_CYAN, RESET);
    println!("  {}[0]{} Cancelar", FG_RED, RESET);
    println!();
    print!(
        "  {}{}Selecciona el componente a desinstalar:{} ",
        FG_WHITE, BOLD, RESET
    );
    let _ = io::stdout().flush();

    // Con la entrada cerrada se cancela: el bucle principal terminara despues.
    let choice = read_line().unwrap_or_default();
    match choice.as_str() {
        "1" => {
            let _ = uninstall_bauh(reporter);
        }
        "2" => {
            let _ = uninstall_gekko_adb(reporter);
        }
        "3" => {
            let _ = uninstall_kito_environment(reporter);
        }
        _ => {
            reporter.info("Desinstalación cancelada.");
        }
    }
}

fn main() {
    // Opciones no interactivas: permiten comprobar la instalacion desde
    // scripts (`gekkoapp --version`) sin entrar en el menu.
    let mut args = std::env::args().skip(1);
    if let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => {
                print_usage();
                return;
            }
            "-V" | "--version" => {
                println!("gekkoapp {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            other => {
                eprintln!("gekkoapp: opcion desconocida: {other}");
                eprintln!("Usa 'gekkoapp --help' para ver las opciones.");
                std::process::exit(2);
            }
        }
    }

    let reporter = CliReporter;
    let environment = SystemEnvironment::detect();
    loop {
        print_banner(&environment);
        print_menu(&environment);

        let Some(option) = read_line() else {
            println!();
            reporter.info("Entrada cerrada (EOF): saliendo de GekkoApp.");
            println!();
            break;
        };

        match option.as_str() {
            "k" | "K" => {
                install_kito_environment(&reporter);
                press_enter_to_continue(&reporter);
            }
            "1" => {
                install_chaotic_aur(&reporter);
                press_enter_to_continue(&reporter);
            }
            "2" => {
                match install_bauh(&reporter, &environment, true) {
                    Ok(()) => {}
                    Err(error) => reporter.err(&format!("Bauh Fork no se instalo: {error}")),
                }
                press_enter_to_continue(&reporter);
            }
            // «b» era la tecla de Gekko ADB antes de renumerar el menu.
            "3" | "b" | "B" => {
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
            "d" | "D" => {
                run_uninstall_menu(&reporter);
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
