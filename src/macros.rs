//! Domain-specific macros for Refinery-RS.

/// Macro to generate YAML blocks with proper escaping for GitHub Actions.
/// Uses a simple placeholder replacement to avoid conflicts with Rust's format! braces.
#[macro_export]
macro_rules! yaml_block {
    ($raw:expr, $($key:ident = $val:expr),* $(,)?) => {
        {
            let mut s = $raw.to_string();
            $(
                s = s.replace(&format!("<{}>", stringify!($key)), &$val.to_string());
            )*
            s
        }
    };
}
