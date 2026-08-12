use std::io::{self, BufRead, Write};
use std::thread;
use std::time::Duration;

pub const RESET: &str = "\x1b[0m";

pub const FG_RED: &str = "\x1b[1;31m";
pub const FG_GREEN: &str = "\x1b[1;32m";
pub const FG_YELLOW: &str = "\x1b[1;33m";
pub const FG_BLUE: &str = "\x1b[1;34m";
pub const FG_MAGENTA: &str = "\x1b[1;35m";
pub const FG_CYAN: &str = "\x1b[1;36m";
pub const FG_WHITE: &str = "\x1b[1;37m";

pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ITALIC: &str = "\x1b[3m";

/// UI-neutral interface used by all core flows.
///
/// The CLI binary uses [`CliReporter`] (terminal output + stdin confirms).
/// The Tauri GUI uses an event-emitting reporter whose confirmations are
/// resolved in the frontend before any command is invoked.
pub trait Reporter: Send + Sync {
    fn ok(&self, msg: &str);
    fn warn(&self, msg: &str);
    fn info(&self, msg: &str);
    fn err(&self, msg: &str);
    fn step(&self, msg: &str);
    fn header(&self, title: &str);
    fn confirm(&self, prompt: &str) -> bool;
    fn prompt(&self, label: &str, current: &str) -> String;
    fn read_line(&self) -> String;
    fn clear_screen(&self);
    fn progress(&self, label: &str, steps: u32);
}

fn clear_screen() {
    print!("\x1b[2J\x1b[H");
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

fn read_line() -> String {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).ok();
    line.trim().to_string()
}

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

/// Reporter that renders to a terminal and reads confirmations from stdin.
#[derive(Debug, Default)]
pub struct CliReporter;

impl Reporter for CliReporter {
    fn ok(&self, msg: &str) {
        println!("{}  ✅  {}{}{}", FG_GREEN, BOLD, msg, RESET);
    }

    fn warn(&self, msg: &str) {
        println!("{}  ⚠️   {}{}{}", FG_YELLOW, BOLD, msg, RESET);
    }

    fn info(&self, msg: &str) {
        println!("{}  ℹ️   {}{}{}", FG_CYAN, BOLD, msg, RESET);
    }

    fn err(&self, msg: &str) {
        println!("{}  ✗   {}{}{}", FG_RED, BOLD, msg, RESET);
    }

    fn step(&self, msg: &str) {
        println!("{}  ➜   {}{}{}", FG_MAGENTA, ITALIC, msg, RESET);
    }

    fn header(&self, title: &str) {
        print_header(title);
    }

    fn confirm(&self, prompt: &str) -> bool {
        print!("  {}  ❓  {} [s/N]:{} ", FG_YELLOW, prompt, RESET);
        let _ = io::stdout().flush();
        let ans = read_line().to_lowercase();
        ans == "s" || ans == "y"
    }

    fn prompt(&self, label: &str, current: &str) -> String {
        print!("  {}{}{} [{}]:{} ", FG_CYAN, BOLD, label, current, RESET);
        let _ = io::stdout().flush();
        let value = read_line();
        if value.is_empty() {
            current.to_string()
        } else {
            value.to_ascii_lowercase()
        }
    }

    fn read_line(&self) -> String {
        read_line()
    }

    fn clear_screen(&self) {
        clear_screen();
    }

    fn progress(&self, label: &str, steps: u32) {
        fake_progress_bar(label, steps);
    }
}
