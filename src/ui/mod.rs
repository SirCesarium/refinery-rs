pub mod components;
pub mod macros;
pub mod prompts;

pub use components::*;

/// User Interface and Interaction module for Refinery.
#[allow(dead_code)]
pub use crate::errors::Result;
pub struct ProgressBarMock;

#[allow(clippy::unused_self)]
impl ProgressBarMock {
    pub const fn inc(&self, _n: u64) {}
    pub const fn finish_with_message(&self, _msg: &str) {}
    pub const fn finish_and_clear(&self) {}
}

#[cfg(feature = "pretty-cli")]
pub use components::BRAND_ORANGE_XTERM;
