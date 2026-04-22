// @swt-disable max-repetition
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
                style.tick_chars("|/-\\")
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
                format!(
                    "{} {{msg}} \x1b[38;5;208m{{bar:30}}\x1b[0m {{percent}}%",
                    "*".color($crate::ui::BRAND_ORANGE_XTERM)
                )
            };
            let style = ProgressStyle::default_bar()
                .template(&template)
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars(if cfg!(feature = "nerd-fonts") { "█▓▒░ " } else { "=>  " });
            pb.set_style(style);
            pb.set_message($msg.bold().to_string());
            pb
        }
        #[cfg(not(feature = "pretty-cli"))]
        {
            println!("[*] {}", $msg);
            $crate::ui::ProgressBarMock
        }
    }};
}

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

#[macro_export]
macro_rules! impl_inquire_text {
    ($msg:expr) => {{
        #[cfg(feature = "pretty-cli")]
        {
            use owo_colors::OwoColorize;
            $msg.color($crate::ui::BRAND_ORANGE_XTERM)
                .bold()
                .to_string()
        }
        #[cfg(not(feature = "pretty-cli"))]
        {
            $msg.to_string()
        }
    }};
}

#[macro_export]
macro_rules! prompt_multi {
    ($msg:expr, $options:expr) => {{
        #[cfg(feature = "pretty-cli")]
        {
            let styled = $crate::impl_inquire_text!($msg);
            inquire::MultiSelect::new(&styled, $options)
                .with_render_config($crate::ui::get_render_config())
                .prompt()
                .map_err(anyhow::Error::from)
        }
        #[cfg(not(feature = "pretty-cli"))]
        {
            let _ = $options;
            Err(anyhow::anyhow!("Multi-prompt offline: {}", $msg))
        }
    }};
}

#[macro_export]
macro_rules! prompt {
    ($msg:expr, $help:expr) => {{
        #[cfg(feature = "pretty-cli")]
        {
            let styled = $crate::impl_inquire_text!($msg);
            inquire::Text::new(&styled)
                .with_render_config($crate::ui::get_render_config())
                .with_help_message($help)
                .prompt()
                .map_err(anyhow::Error::from)
        }
        #[cfg(not(feature = "pretty-cli"))]
        {
            let _ = $help;
            Err(anyhow::anyhow!("Prompt offline: {}", $msg))
        }
    }};

    ($msg:expr) => {
        $crate::prompt!($msg, "Required")
    };
}
