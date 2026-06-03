mod domain;

pub use domain::{LrcDocument, LrcParser, LyricEntry, SelectionMode, SelectionState};

#[cfg(target_arch = "wasm32")]
mod web_app;

#[cfg(target_arch = "wasm32")]
pub fn run() {
    web_app::run();
}
