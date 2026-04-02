//! User Interface and Interaction module.
//! Handles terminal output and interactive prompts.

use crate::errors::Result;
use console::Style;
use inquire::{Confirm, Text};

/// Internal macro to handle stylized printing.
#[macro_export]
macro_rules! log_step {
    ($icon:expr, $color:ident, $($arg:tt)*) => {
        println!("{} {}", $icon, console::Style::new().$color().bold().apply_to(format!($($arg)*)));
    };
}

/// Prints a success message.
pub fn success(msg: &str) {
    println!("[+] {}", Style::new().green().bold().apply_to(msg));
}

/// Prints an informational message.
pub fn info(msg: &str) {
    println!("[i] {}", Style::new().cyan().apply_to(msg));
}

/// Prints a warning message.
pub fn warn(msg: &str) {
    println!("[!] {}", Style::new().yellow().apply_to(msg));
}

/// Wrapper for confirmation prompts.
pub fn prompt_confirm(msg: &str, default: bool) -> Result<bool> {
    Ok(Confirm::new(msg).with_default(default).prompt()?)
}

/// Wrapper for text input prompts.
pub fn prompt_text(msg: &str, default: &str) -> Result<String> {
    Ok(Text::new(msg).with_default(default).prompt()?)
}

/// Prints the ASCII banner for the CLI.
pub fn print_banner() {
    let orange = Style::new().color256(208);
    let banner = r"
    ____       _____                      
   / __ \___  / __(_)___  ___  _______  __
  / /_/ / _ \/ /_/ / __ \/ _ \/ ___/ / / /
 / _, _/  __/ __/ / / / /  __/ /  / /_/ / 
/_/ |_|\___/_/ /_/_/ /_/\___/_/   \__, /  
                                 /____/  
    ";
    println!("{}", orange.apply_to(banner));
    println!("🦀 Refining Rust into universal artifacts.\n");
}
