//! User Interface and Interaction module.
#![allow(dead_code)]

use crate::errors::Result;
use console::{Style, style};
use inquire::{
    Confirm, Select, Text,
    ui::{Attributes, Color, RenderConfig, StyleSheet, Styled},
};
use std::fmt::Display;

const BRAND_ORANGE: Style = Style::new().color256(208);

fn get_render_config() -> RenderConfig<'static> {
    RenderConfig {
        help_message: StyleSheet::new()
            .with_fg(Color::LightYellow)
            .with_attr(Attributes::ITALIC),

        prompt_prefix: Styled::new("?").with_fg(Color::LightRed),

        answered_prompt_prefix: Styled::new("✓").with_fg(Color::LightYellow),

        highlighted_option_prefix: Styled::new("> ").with_fg(Color::LightRed),

        selected_option: Some(StyleSheet::new().with_fg(Color::LightYellow)),

        answer: StyleSheet::new()
            .with_fg(Color::LightRed)
            .with_attr(Attributes::BOLD),

        ..RenderConfig::default()
    }
}

/// Prints a stylized step with a custom icon and color.
#[macro_export]
macro_rules! log_step {
    ($icon:expr, $color:ident, $($arg:tt)*) => {
        println!("{} {}", $icon, console::Style::new().$color().bold().apply_to(format!($($arg)*)));
    };
}

pub fn success(msg: &str) {
    println!("{} {}", style("[+]").green().bold(), msg);
}

pub fn info(msg: &str) {
    println!("{} {}", style("[i]").yellow().bold(), msg);
}

pub fn warn(msg: &str) {
    println!("{} {}", BRAND_ORANGE.bold().apply_to("[!]"), msg);
}

pub fn inquire_text(msg: &str) -> String {
    BRAND_ORANGE.apply_to(msg).to_string()
}

pub fn prompt_opt<T: Display>(msg: &str, options: Vec<T>) -> Result<T> {
    Ok(Select::new(&inquire_text(msg), options)
        .with_render_config(get_render_config())
        .prompt()?)
}

pub fn prompt_confirm(msg: &str, default: bool) -> Result<bool> {
    Ok(Confirm::new(&inquire_text(msg))
        .with_default(default)
        .with_render_config(get_render_config())
        .prompt()?)
}

pub fn prompt_def(msg: &str, default: &str) -> Result<String> {
    Ok(Text::new(&inquire_text(msg))
        .with_default(default)
        .with_render_config(get_render_config())
        .prompt()?)
}

pub fn prompt(msg: &str) -> Result<String> {
    Ok(Text::new(&inquire_text(msg))
        .with_render_config(get_render_config())
        .prompt()?)
}

pub fn print_banner() {
    let banner = r"
    ____       _____                      
   / __ \___  / __(_)___  ___  _______  __
  / /_/ / _ \/ /_/ / __ \/ _ \/ ___/ / / /
 / _, _/  __/ __/ / / / /  __/ /  / /_/ / 
/_/ |_|\___/_/ /_/_/ /_/\___/_/   \__, /  
                                 /____/  ";

    println!("{}", BRAND_ORANGE.apply_to(banner));
    println!("🦀 Refining Rust into universal artifacts.\n");
}
