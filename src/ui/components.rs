use crate::errors::Result;
use std::fmt::Display;

#[cfg(feature = "pretty-cli")]
use inquire::ui::{Attributes, Color as InquireColor, RenderConfig, StyleSheet, Styled};

#[cfg(feature = "pretty-cli")]
pub const BRAND_ORANGE_XTERM: owo_colors::XtermColors = owo_colors::XtermColors::FlushOrange;

#[cfg(feature = "pretty-cli")]
#[must_use]
pub fn get_render_config() -> RenderConfig<'static> {
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
            "✓"
        })
        .with_fg(InquireColor::DarkGrey),
        highlighted_option_prefix: Styled::new(if cfg!(feature = "nerd-fonts") {
            " 󱞩 "
        } else {
            " > "
        })
        .with_fg(InquireColor::LightCyan),
        selected_checkbox: Styled::new(if cfg!(feature = "nerd-fonts") {
            "󰄬 "
        } else {
            "[x] "
        })
        .with_fg(InquireColor::LightGreen),
        unselected_checkbox: Styled::new(if cfg!(feature = "nerd-fonts") {
            "󰄱 "
        } else {
            "[ ] "
        })
        .with_fg(InquireColor::Grey),
        selected_option: Some(
            StyleSheet::new()
                .with_fg(InquireColor::LightCyan)
                .with_attr(Attributes::BOLD),
        ),
        answer: StyleSheet::new()
            .with_fg(InquireColor::White)
            .with_attr(Attributes::BOLD),
        ..RenderConfig::default()
    }
}

pub fn success(msg: &str) {
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        let icon = if cfg!(feature = "nerd-fonts") {
            "󰄬"
        } else {
            "✓"
        };
        println!("{} {}", icon.green().bold(), msg.dimmed());
    }
    #[cfg(not(feature = "pretty-cli"))]
    println!("✓ {msg}");
}

pub fn info(msg: &str) {
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        let icon = if cfg!(feature = "nerd-fonts") {
            "󰋼"
        } else {
            "i"
        };
        println!("{} {}", icon.cyan().bold(), msg.dimmed());
    }
    #[cfg(not(feature = "pretty-cli"))]
    println!("i {msg}");
}

pub fn warn(msg: &str) {
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        let icon = if cfg!(feature = "nerd-fonts") {
            "󰀪"
        } else {
            "!"
        };
        println!("{} {}", icon.color(BRAND_ORANGE_XTERM).bold(), msg);
    }
    #[cfg(not(feature = "pretty-cli"))]
    println!("! {msg}");
}

#[must_use]
pub fn inquire_text(msg: &str) -> String {
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        msg.white().to_string()
    }
    #[cfg(not(feature = "pretty-cli"))]
    msg.to_string()
}

/// # Errors
/// Returns error if prompt fails.
pub fn prompt_opt<T: Display>(msg: &str, options: Vec<T>) -> Result<T> {
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        let styled_msg = msg.color(BRAND_ORANGE_XTERM).bold().to_string();

        Ok(inquire::Select::new(&styled_msg, options)
            .with_render_config(get_render_config())
            .with_page_size(10)
            .prompt()?)
    }
    #[cfg(not(feature = "pretty-cli"))]
    {
        let _ = options;
        Err(crate::errors::RefineryError::Config(format!(
            "Prompt offline: {msg}"
        )))
    }
}

pub fn error(err: &anyhow::Error) {
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        let icon = if cfg!(feature = "nerd-fonts") {
            " 󰅙"
        } else {
            " X"
        };
        eprintln!(
            "\n{}{}",
            icon.on_red(),
            " CRITICAL ".black().on_red().bold()
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
        eprintln!("  {} {} \n", "└─>".dimmed(), err);
    }
    #[cfg(not(feature = "pretty-cli"))]
    eprintln!("ERROR: {err:?}");
}

/// # Errors
/// Returns error if prompt fails.
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
        println!("{msg} [y/n]: {default}");
        Ok(default)
    }
}

pub fn print_banner() {
    #[cfg(feature = "pretty-cli")]
    {
        use owo_colors::OwoColorize;
        let banner = r"
              _____                      
   ________  / __(_)___  ___  _______  __
  / ___/ _ \/ /_/ / __ \/ _ \/ ___/ / / /
 / /  /  __/ __/ / / / /  __/ /  / /_/ / 
/_/   \___/_/ /_/_/ /_/\___/_/   \__, /  
                                /____/   ";
        println!(
            "{}\n\n{}",
            banner.color(BRAND_ORANGE_XTERM),
            "🦀 Refining Rust into universal artifacts.\n".dimmed()
        );
    }
    #[cfg(not(feature = "pretty-cli"))]
    println!("--- Refinery-RS ---");
}

pub fn print_highlighted_toml(content: &str) {
    for line in content.lines() {
        if line.starts_with('[') {
            println!("\x1b[38;5;208m{line}\x1b[0m");
        } else if line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            let key = parts[0];
            let val = parts.get(1).unwrap_or(&"");
            println!("\x1b[36m{key}\x1b[0m=\x1b[32m{val}\x1b[0m");
        } else {
            println!("{line}");
        }
    }
}
