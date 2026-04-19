//! User Interface and Interaction module for Refinery.
// @swt-disable max-lines max-repetition
#![allow(dead_code)]

use crate::errors::Result;
use std::fmt::Display;

#[cfg(feature = "pretty-cli")]
use inquire::ui::{Attributes, Color as InquireColor, RenderConfig, StyleSheet, Styled};

#[cfg(feature = "pretty-cli")]
pub const BRAND_ORANGE_XTERM: owo_colors::XtermColors = owo_colors::XtermColors::FlushOrange;

/// Mock structure to maintain compilation when pretty-cli is disabled.
pub struct ProgressBarMock;

#[allow(clippy::unused_self)]
impl ProgressBarMock {
    /// Increments the mock progress.
    pub const fn inc(&self, _n: u64) {}
    /// Finishes the mock progress with a message.
    pub const fn finish_with_message(&self, _msg: &str) {}
    /// Finishes and clears the mock progress.
    pub const fn finish_and_clear(&self) {}
}

#[cfg(feature = "pretty-cli")]
fn get_render_config() -> RenderConfig<'static> {
    RenderConfig {
        help_message: StyleSheet::new()
            .with_fg(InquireColor::DarkGrey)
            .with_attr(Attributes::ITALIC),
        prompt_prefix: Styled::new(if cfg!(feature = "nerd-fonts") {
            "󱩔"
        } else {
            ">"
        })
        .with_fg(InquireColor::Grey),
        answered_prompt_prefix: Styled::new(if cfg!(feature = "nerd-fonts") {
            "󰒓"
        } else {
            "!"
        })
        .with_fg(InquireColor::DarkGrey),
        highlighted_option_prefix: Styled::new(if cfg!(feature = "nerd-fonts") {
            "󱞩 "
        } else {
            ">> "
        })
        .with_fg(InquireColor::LightRed),
        selected_option: Some(StyleSheet::new().with_fg(InquireColor::White)),
        answer: StyleSheet::new()
            .with_fg(InquireColor::White)
            .with_attr(Attributes::BOLD),
        ..RenderConfig::default()
    }
}

/// Progress bar and spinner macro.
#[macro_export]
macro_rules! spinner {
    ($msg:expr) => {{
        #[cfg(feature = "pretty-cli")]
        {
            use indicatif::{ProgressBar, ProgressStyle};
            use owo_colors::OwoColorize;
            use std::io::{Write, stdout};
            let pb = ProgressBar::new_spinner();
            let style = ProgressStyle::default_spinner()
                .template("{spinner} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner());
            let style = if cfg!(feature = "nerd-fonts") {
                style.tick_strings(&[
                    &"󰈸  ".color($crate::ui::BRAND_ORANGE_XTERM).to_string(),
                    &"󰈸󰈸 ".color($crate::ui::BRAND_ORANGE_XTERM).to_string(),
                    &"󰈸󰈸󰈸".color($crate::ui::BRAND_ORANGE_XTERM).to_string(),
                    &"󰈸󰈸󰈸".color($crate::ui::BRAND_ORANGE_XTERM).to_string(),
                    &" 󰈸󰈸".color($crate::ui::BRAND_ORANGE_XTERM).to_string(),
                    &"  󰈸".color($crate::ui::BRAND_ORANGE_XTERM).to_string(),
                ])
            } else {
                style.tick_chars(&"⠁⠂⠄⡀⢀⠠⠐⠈".color($crate::ui::BRAND_ORANGE_XTERM).to_string())
            };
            pb.set_style(style);
            pb.set_message($msg.bold().to_string());
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            let _ = stdout().flush();
            pb
        }
        #[cfg(not(feature = "pretty-cli"))]
        {
            println!("... {}", $msg);
            $crate::ui::ProgressBarMock
        }
    }};
}

/// Progress bar for determinate tasks.
#[macro_export]
macro_rules! progress {
    ($len:expr, $msg:expr) => {{
        #[cfg(feature = "pretty-cli")]
        {
            use indicatif::{ProgressBar, ProgressStyle};
            use owo_colors::OwoColorize;
            let pb = ProgressBar::new($len);
            let template = if cfg!(feature = "nerd-fonts") {
                format!(
                    "{} {{msg}} \x1b[38;5;208m{{bar:30}}\x1b[0m {{percent}}% \x1b[2m({{pos}}/{{len}})\x1b[0m",
                    "󰒓".color($crate::ui::BRAND_ORANGE_XTERM)
                )
            } else {
                format!("{{msg}} \x1b[38;5;208m{{bar:30}}\x1b[0m {{percent}}%")
            };
            let style = ProgressStyle::default_bar()
                .template(&template)
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars(if cfg!(feature = "nerd-fonts") { "█▓▒░ " } else { "■□ " });
            pb.set_style(style);
            pb.set_message($msg.bold().to_string());
            pb
        }
        #[cfg(not(feature = "pretty-cli"))]
        {
            println!("[REFINING] {}", $msg);
            $crate::ui::ProgressBarMock
        }
    }};
}

/// Prints a stylized step with a custom icon.
#[macro_export]
macro_rules! log_step {
    ($icon_nf:expr, $icon_plain:expr, $color:ident, $($arg:tt)*) => {
        #[cfg(feature = "pretty-cli")]
        {
            use owo_colors::OwoColorize;
            let icon = if cfg!(feature = "nerd-fonts") { $icon_nf } else { $icon_plain };
            println!("{} {}", icon.color($crate::ui::BRAND_ORANGE_XTERM).bold(), format!($($arg)*).white());
        }
        #[cfg(not(feature = "pretty-cli"))]
        {
            println!("{} {}", $icon_plain, format!($($arg)*));
        }
    };
}

macro_rules! impl_log {
    ($name:ident, $nf_icon:expr, $plain_icon:expr, $color:ident) => {
        pub fn $name(msg: &str) {
            #[cfg(feature = "pretty-cli")]
            {
                use owo_colors::OwoColorize;
                let icon = if cfg!(feature = "nerd-fonts") {
                    $nf_icon
                } else {
                    $plain_icon
                };
                println!("{} {}", icon.$color().bold(), msg.dimmed());
            }
            #[cfg(not(feature = "pretty-cli"))]
            {
                println!("{} {}", $plain_icon, msg);
            }
        }
    };
}

impl_log!(success, "󰄬", "[OK]", green);
impl_log!(info, "󰋼", "[II]", white);

/// Prints a warning message.
pub fn warn(msg: &str) {
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        let icon = if cfg!(feature = "nerd-fonts") {
            "󰀪"
        } else {
            "[!]"
        };
        println!("{} {}", icon.color(BRAND_ORANGE_XTERM).bold(), msg);
    }
    #[cfg(not(feature = "pretty-cli"))]
    println!("[!] {}", msg);
}

/// Styles text for interactive prompts.
pub fn inquire_text(msg: &str) -> String {
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        msg.white().to_string()
    }
    #[cfg(not(feature = "pretty-cli"))]
    msg.to_string()
}

/// Prompts the user to select an option from a list.
pub fn prompt_opt<T: Display>(msg: &str, options: Vec<T>) -> Result<T> {
    #[cfg(feature = "pretty-cli")]
    {
        Ok(inquire::Select::new(&inquire_text(msg), options)
            .with_render_config(get_render_config())
            .prompt()?)
    }
    #[cfg(not(feature = "pretty-cli"))]
    {
        let _ = options;
        Err(crate::errors::RefineryError::Config(format!(
            "Prompt offline: {}",
            msg
        )))
    }
}

/// Formats and prints a critical error to stderr.
pub fn error(err: &anyhow::Error) {
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        let icon = if cfg!(feature = "nerd-fonts") {
            " 󰅙 "
        } else {
            "XX "
        };
        eprintln!(
            "\n{}{} {}",
            icon.on_red(),
            " CRITICAL ".black().on_red().bold(),
            err.to_string().red()
        );
        let mut current = err.source();
        while let Some(cause) = current {
            let prefix = if cfg!(feature = "nerd-fonts") {
                "│"
            } else {
                "|"
            };
            eprintln!("  {} {}", prefix.dimmed(), cause.to_string().dimmed());
            current = cause.source();
        }
        eprintln!("  {} \n", "└─".dimmed());
    }
    #[cfg(not(feature = "pretty-cli"))]
    eprintln!("ERROR: {:?}", err);
}

/// Prompts the user for a boolean confirmation.
pub fn prompt_confirm(msg: &str, default: bool) -> Result<bool> {
    #[cfg(feature = "pretty-cli")]
    {
        Ok(inquire::Confirm::new(&inquire_text(msg))
            .with_default(default)
            .with_render_config(get_render_config())
            .prompt()?)
    }
    #[cfg(not(feature = "pretty-cli"))]
    {
        println!("{} [y/n]: {}", msg, default);
        Ok(default)
    }
}

/// Prompts the user for mandatory text input.
pub fn prompt(msg: &str) -> Result<String> {
    #[cfg(feature = "pretty-cli")]
    {
        Ok(inquire::Text::new(&inquire_text(msg))
            .with_render_config(get_render_config())
            .prompt()?)
    }
    #[cfg(not(feature = "pretty-cli"))]
    Err(crate::errors::RefineryError::Config(msg.into()))
}

/// Displays the project ASCII banner.
#[allow(clippy::used_underscore_binding)]
pub fn print_banner() {
    let _banner = r"
              _____                      
   ________  / __(_)___  ___  _______  __
  / ___/ _ \/ /_/ / __ \/ _ \/ ___/ / / /
 / /  /  __/ __/ / / / /  __/ /  / /_/ / 
/_/   \___/_/ /_/_/ /_/\___/_/   \__, /  
                                /____/   ";
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        println!(
            "{}\n\n{}",
            _banner.color(BRAND_ORANGE_XTERM),
            "🦀 Refining Rust into universal artifacts.\n".dimmed()
        );
    }
    #[cfg(not(feature = "pretty-cli"))]
    println!("--- Refinery-RS ---");
}
