mod domain;

pub use domain::{LrcDocument, LrcParser, LyricEntry, SelectionMode, SelectionState, TimeMs, Pixels};

#[cfg(any(target_arch = "wasm32", test))]
pub mod web_app;

#[cfg(target_arch = "wasm32")]
pub fn run() {
    web_app::run();
}
