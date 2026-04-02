//! Domain-specific macros for Refinery-RS.

/// Macro to generate YAML blocks with proper escaping for GitHub Actions.
/// Handles double braces automatically for the user.
#[macro_export]
macro_rules! yaml_block {
    ($($t:tt)*) => {
        format!($($t)*)
            .replace("${{", "${{{{")
            .replace("}}", "}}}}")
    };
}
