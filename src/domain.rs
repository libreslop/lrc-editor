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
pub use time::{TimeMs, Pixels};
