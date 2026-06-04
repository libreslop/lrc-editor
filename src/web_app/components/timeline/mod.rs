pub mod waveform_canvas;
pub mod playback_controls;
pub mod track_pads;
pub mod lyric_chunk;
pub mod timeline_lanes;

pub use waveform_canvas::{WaveformCanvas, WaveformSummary};
pub use playback_controls::PlaybackControls;
pub use track_pads::TrackPads;
pub use lyric_chunk::LyricChunk;
pub use timeline_lanes::TimelineLanes;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DragTarget {
    Body,
    LeftEdge,
    RightEdge,
    Boundary,
    Playhead,
    Selection,
    Scrollbar,
}
