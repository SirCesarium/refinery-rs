// @swt-disable max-repetition max-depth

/// Defines a top-level command logic.
#[macro_export]
macro_rules! cmd {
    ($name:ident($($arg:ident: $typ:ty),*) $body:block) => {
        impl $crate::commands::Actions {
            pub async fn $name($($arg: $typ),*) -> miette::Result<()> {
                $body
            }
        }
    };
}

/// Defines logic for a variant using a prefixed method to avoid name collision.
#[macro_export]
macro_rules! subcmd {
    ($enum_ty:ty, $variant:ident($($arg:ident: $typ:ty),*) $body:block) => {
        impl $enum_ty {
            paste::paste! {
                pub async fn [<run_ $variant:snake>]($($arg: $typ),*) -> miette::Result<()> {
                    $body
                }
            }
        }
    };
}

/// Automatically dispatches subcommands by calling the prefixed methods.
#[macro_export]
macro_rules! auto_dispatch {
    ($target:expr, $enum_ty:ty, { $($variant:ident $(($arg:ident))?),* $(,)? }) => {
        {
            use $enum_ty::*;
            match $target {
                $(
                    $variant $(($arg))? => {
                        paste::paste! {
                            <$enum_ty>::[< run_ $variant:snake >]($($arg)?).await
                        }
                    },
                )*
            }
        }
    };
}
