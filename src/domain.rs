pub mod time;
pub mod metadata;
pub mod entry;
pub mod document;
pub mod parser;
pub mod selection;

pub use entry::LyricEntry;
pub use document::LrcDocument;
pub use parser::LrcParser;
pub use selection::{SelectionState, SelectionMode};
pub use time::{TimeMs, Pixels, ZoomLevel};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
