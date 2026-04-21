use crate::ui::{Result, get_render_config};
use inquire::Select;

fn select_menu(msg: &str, options: Vec<&'static str>) -> Result<String> {
    Ok(Select::new(msg, options)
        .with_render_config(get_render_config())
        .prompt()?
        .to_string())
}

/// # Errors
/// Returns error if prompt fails.
pub fn select_main_action() -> Result<String> {
    select_menu(
        "Main Menu",
        vec!["Binaries", "Libraries", "Targets", "Review & Save", "Exit"],
    )
}

/// # Errors
/// Returns error if prompt fails.
pub fn select_init_action() -> Result<String> {
    select_menu(
        "Main Menu:",
        vec![
            "Add Binaries",
            "Add Libraries",
            "Configure Targets",
            "Review & Save",
        ],
    )
}
