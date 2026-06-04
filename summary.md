# Refactoring Summary

## What I have done
1. **Type Integration (`TimeMs`, `Pixels`)**:
   - Unified `TimeMs` and `Milliseconds` into a single `TimeMs` type in `src/domain/time.rs`.
   - Replaced raw `u32` (time) and `f64` (pixels) values with `TimeMs` and `Pixels` semantic wrappers across the entire application, including domain models (`LyricEntry`, `TimelineChunk`, `LrcDocument`), state (`AppState`, `AppAction`), and components (`TimelinePanel`, etc.).
   - Added helper methods to `TimeMs` and `Pixels` for better ergonomics (e.g., `to_secs()`, `as_timestamp()`).

2. **`TimelinePanel` Refactoring**:
   - Decomposed the monolithic `src/web_app/components/timeline_panel.rs` into smaller, reusable Yew components under `src/web_app/components/timeline/`.
   - **New Components**:
     - `WaveformCanvas`: Encapsulates the waveform rendering logic.
     - `PlaybackControls`: Manages transport and zoom UI.
     - `TrackPads`: Labels and actions for the timeline lanes.
     - `TimelineLanes`: Orchestrates the ruler, audio, and lyric lanes.
     - `LyricChunk`: Individual interactive lyric segment on the timeline.
   - This significantly reduced the complexity of `TimelinePanel` and improved code maintainability.

3. **State Management Refactoring**:
   - Extracted `AppAction` and the reducer logic from `src/web_app/app.rs` into a dedicated `src/web_app/actions.rs` file.
   - This separates state transition logic from the main `App` component, following better architectural practices.

## What needs to be fixed / Next Steps
1. **Unused Code Cleanup**:
   - There are several dead code warnings (e.g., `AudioPlayer` in `audio.rs`, some fields in `Interval`). These should be reviewed and removed if they are indeed legacy or no longer needed.
2. **Test Coverage**:
   - While the refactoring has been verified with `trunk build`, the project could benefit from more extensive unit tests, especially for the new `TimeMs` methods and the state reducer in `actions.rs`.
3. **UI Polishing**:
   - Now that the components are split, it's easier to add more refined interactions or styling to individual parts of the timeline.

## How to continue
1. Run `NO_COLOR=true trunk build --release` to ensure everything still compiles.
2. Address the remaining dead code warnings in `audio.rs` and `editor/timeline.rs`.
3. Consider implementing more robust error handling in the `WaveformCanvas` fetch/decode logic.
