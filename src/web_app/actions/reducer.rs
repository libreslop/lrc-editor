use std::rc::Rc;
use yew::prelude::*;
use super::models::AppState;
use super::types::AppAction;
use crate::domain::{SelectionState, TimeMs, ZoomLevel, LrcDocument, SelectionMode};

impl Reducible for AppState {
    type Action = AppAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut new_state = (*self).clone();
        match action {
            AppAction::UpdateSource(source) => {
                new_state.update_document(source);
            }
            AppAction::SetLrcFilename(name) => {
                new_state.project.lrc_filename = Some(name);
            }
            AppAction::SetAudioFilename(name) => {
                new_state.project.audio_filename = Some(name);
            }
            AppAction::SelectEntry(id, mode) => {
                if let Some(doc) = &new_state.document.document {
                    new_state.view.selection.select_entry(doc, id, mode);
                }
            }
            AppAction::ClearSelection => {
                new_state.view.selection = SelectionState::default();
            }
            AppAction::SelectAll => {
                if let Some(doc) = &new_state.document.document {
                    new_state.view.selection.select_all(doc);
                }
            }
            AppAction::SetTime(time) => {
                let max_dur = new_state.max_timeline_duration();
                let bounded_time = if time.as_u32() > max_dur.as_u32() {
                    max_dur
                } else {
                    time
                };

                if !new_state.playback.playing {
                    new_state.playback.last_seek_request = None;
                    new_state.playback.current_time_ms = bounded_time;
                } else {
                    if let Some(seek_time) = new_state.playback.last_seek_request {
                        if bounded_time == seek_time {
                            new_state.playback.last_seek_request = None;
                            new_state.playback.current_time_ms = bounded_time;
                        } else {
                            // Ignore stale updates from active playback interval ticks
                            // while we have a pending seek request.
                            return self;
                        }
                    } else {
                        new_state.playback.current_time_ms = bounded_time;
                    }
                }
            }
            AppAction::SetDuration(time) => {
                new_state.playback.duration_ms = time;
                let max_dur = new_state.max_timeline_duration();
                if new_state.playback.current_time_ms.as_u32() > max_dur.as_u32() {
                    new_state.playback.current_time_ms = max_dur;
                }
            }
            AppAction::TogglePlay => {
                new_state.playback.playing = !new_state.playback.playing;
                new_state.playback.last_seek_request = None;
            }
            AppAction::Seek(time) => {
                let max_dur = new_state.max_timeline_duration();
                let clamped_time = if time.as_u32() > max_dur.as_u32() {
                    max_dur
                } else {
                    time
                };
                new_state.playback.last_seek_request = Some(clamped_time);
                new_state.playback.current_time_ms = clamped_time;
            }
            AppAction::Undo => {
                if new_state.history.history_index > 0 {
                    new_state.history.history_index -= 1;
                    let source = new_state.history.history[new_state.history.history_index].clone();
                    new_state.update_document(source);
                }
            }
            AppAction::Redo => {
                if new_state.history.history_index + 1 < new_state.history.history.len() {
                    new_state.history.history_index += 1;
                    let source = new_state.history.history[new_state.history.history_index].clone();
                    new_state.update_document(source);
                }
            }
            AppAction::SetZoom(zoom) => {
                new_state.view.zoom_level = ZoomLevel(zoom.as_f64().clamp(0.001, 10.0));
            }
            AppAction::SaveHistory(source) => {
                new_state.history.history.truncate(new_state.history.history_index + 1);
                new_state.history.history.push(source);
                new_state.history.history_index = new_state.history.history.len() - 1;
            }
            AppAction::DeleteSelected => {
                if !new_state.view.selection.selected_ids().is_empty()
                    && let Some(doc) = &new_state.document.document
                {
                    let selected_uids = new_state.view.selection.selected_ids().to_vec();
                    let mut entries = doc.entries().to_vec();
                    
                    for entry in entries.iter_mut() {
                        if selected_uids.contains(&entry.uid()) {
                            entry.text = String::new();
                            entry.display_text = String::new();
                        }
                    }
                    
                    let mut merged_entries: Vec<crate::domain::LyricEntry> = Vec::new();
                    for entry in entries {
                        if let Some(last) = merged_entries.last()
                            && last.is_empty() && entry.is_empty()
                        {
                            continue;
                        }
                        merged_entries.push(entry);
                    }
                    let entries = merged_entries;
                    
                    let new_doc = LrcDocument::new(entries, doc.metadata().to_vec(), doc.line_count());
                    let text = new_doc.to_source_text();
                    
                    new_state.update_document(text.clone());
                    
                    new_state.history.history.truncate(new_state.history.history_index + 1);
                    new_state.history.history.push(text);
                    new_state.history.history_index = new_state.history.history.len() - 1;
                }
            }
            AppAction::ShiftSelected(delta_ms) => {
                if !new_state.view.selection.selected_ids().is_empty() && delta_ms != 0
                    && let Some(doc) = &new_state.document.document
                {
                    let timeline_duration_ms = new_state.max_timeline_duration();
                    let editor = crate::web_app::editor::timeline::TimelineEditor::new(doc);
                    let text = editor.shift_selected(
                        new_state.view.selection.selected_ids(),
                        delta_ms,
                        timeline_duration_ms
                    );
                    
                    new_state.update_document(text.clone());
                    new_state.history.history.truncate(new_state.history.history_index + 1);
                    new_state.history.history.push(text);
                    new_state.history.history_index = new_state.history.history.len() - 1;
                }
            }
            AppAction::ShiftBoundary(chunk_id, left_edge, both, delta_ms) => {
                if delta_ms != 0
                    && let Some(doc) = &new_state.document.document
                {
                    let timeline_duration_ms = new_state.max_timeline_duration();
                    let editor = crate::web_app::editor::timeline::TimelineEditor::new(doc);
                    let text = editor.shift_boundary(
                        chunk_id,
                        left_edge,
                        both,
                        delta_ms,
                        timeline_duration_ms
                    );
                    
                    new_state.update_document(text.clone());
                    new_state.history.history.truncate(new_state.history.history_index + 1);
                    new_state.history.history.push(text);
                    new_state.history.history_index = new_state.history.history.len() - 1;
                }
            }
            AppAction::AddChunk(start, end) => {
                if start < end {
                    let doc_to_use = new_state.document.document.clone().unwrap_or_else(|| {
                        LrcDocument::new(vec![], vec![], 0)
                    });
                    let timeline_duration_ms = new_state.max_timeline_duration();
                    let editor = crate::web_app::editor::timeline::TimelineEditor::new(&doc_to_use);
                    let text = editor.add_chunk(start, end, timeline_duration_ms);
                    
                    let new_uid = new_state.document.next_uid;
                    
                    new_state.update_document(text.clone());
                    new_state.history.history.truncate(new_state.history.history_index + 1);
                    new_state.history.history.push(text);
                    new_state.history.history_index = new_state.history.history.len() - 1;
                    
                    if let Some(new_doc) = &new_state.document.document {
                        let to_select = new_doc.entries().iter()
                            .filter(|e| e.display_text() == "*CHANGE ME*")
                            .min_by_key(|e| {
                                (e.time_ms().as_u32() as i32 - start.as_u32() as i32).abs()
                            })
                            .map(|e| e.uid())
                            .unwrap_or_else(|| {
                                new_doc.entries().iter()
                                    .find(|e| e.uid() >= new_uid)
                                    .map(|e| e.uid())
                                    .unwrap_or(new_uid)
                            });
                        new_state.view.selection.select_entry(new_doc, to_select, SelectionMode::Replace);
                    }
                }
            }
        }
        
        let was_empty = self.document.document.as_ref()
            .is_none_or(|doc| doc.entries().iter().all(|entry| entry.is_empty()));
        let is_empty = new_state.document.document.as_ref()
            .is_none_or(|doc| doc.entries().iter().all(|entry| entry.is_empty()));
        if is_empty && !was_empty {
            new_state.playback.current_time_ms = TimeMs(0);
        }

        Rc::new(new_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_state() -> Rc<AppState> {
        Rc::new(AppState::default())
    }

    #[test]
    fn test_reduce_update_source() {
        let state = mock_state();
        let new_state = state.reduce(AppAction::UpdateSource("[00:01.00]Test".to_string()));
        assert_eq!(new_state.document.source_text, "[00:01.00]Test");
        assert!(new_state.document.document.is_some());
        assert!(new_state.document.parse_error.is_none());
    }

    #[test]
    fn test_reduce_seek() {
        let state = mock_state();
        let new_state = state.reduce(AppAction::Seek(TimeMs(1000)));
        assert_eq!(new_state.playback.current_time_ms, TimeMs(1000));
        assert_eq!(new_state.playback.last_seek_request, Some(TimeMs(1000)));
    }

    #[test]
    fn test_reduce_zoom() {
        let state = mock_state();
        let new_state = state.clone().reduce(AppAction::SetZoom(ZoomLevel(2.0)));
        assert_eq!(new_state.view.zoom_level, ZoomLevel(2.0));
        
        let clamped = state.reduce(AppAction::SetZoom(ZoomLevel(100.0)));
        assert_eq!(clamped.view.zoom_level, ZoomLevel(10.0));
    }

    #[test]
    fn test_duration_clamping() {
        let state = mock_state();
        // Default duration is 0, so max_timeline_duration is 15000ms (overscroll)
        
        let seek_far = state.clone().reduce(AppAction::Seek(TimeMs(25000)));
        assert_eq!(seek_far.playback.current_time_ms, TimeMs(15000));
        
        let set_dur = state.clone().reduce(AppAction::SetDuration(TimeMs(5000)));
        // max_timeline_duration becomes 20000ms
        let seek_edge = set_dur.reduce(AppAction::Seek(TimeMs(20000)));
        assert_eq!(seek_edge.playback.current_time_ms, TimeMs(20000));
    }
}
