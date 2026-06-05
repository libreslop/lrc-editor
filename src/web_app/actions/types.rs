use crate::domain::{SelectionMode, TimeMs, ZoomLevel};

#[derive(Clone, PartialEq)]
pub enum AppAction {
    UpdateSource(String),
    SetLrcFilename(String),
    SetAudioFilename(String),
    SelectEntry(usize, SelectionMode),
    ClearSelection,
    SelectAll,
    SetTime(TimeMs),
    SetDuration(TimeMs),
    TogglePlay,
    Seek(TimeMs),
    Undo,
    Redo,
    SetZoom(ZoomLevel),
    SaveHistory(String),
    DeleteSelected,
    ShiftSelected(i32),
    ShiftBoundary(usize, bool, bool, i32), // id, is_left, both, delta
    AddChunk(TimeMs, TimeMs),
}
