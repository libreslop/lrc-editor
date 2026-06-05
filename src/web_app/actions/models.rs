use crate::domain::{LrcDocument, LrcParser, SelectionState, TimeMs, ZoomLevel};
use wasm_bindgen::JsCast;

#[derive(PartialEq, Clone)]
pub struct PlaybackState {
    pub current_time_ms: TimeMs,
    pub duration_ms: TimeMs,
    pub playing: bool,
    pub last_seek_request: Option<TimeMs>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            current_time_ms: TimeMs(0),
            duration_ms: TimeMs(0),
            playing: false,
            last_seek_request: None,
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct DocumentState {
    pub source_text: String,
    pub document: Option<LrcDocument>,
    pub parse_error: Option<String>,
    pub next_uid: usize,
}

impl Default for DocumentState {
    fn default() -> Self {
        Self {
            source_text: String::new(),
            document: None,
            parse_error: None,
            next_uid: 1,
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct HistoryState {
    pub history: Vec<String>,
    pub history_index: usize,
}

impl Default for HistoryState {
    fn default() -> Self {
        Self {
            history: vec![String::new()],
            history_index: 0,
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct ViewState {
    pub zoom_level: ZoomLevel,
    pub selection: SelectionState,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            zoom_level: ZoomLevel(0.25),
            selection: SelectionState::default(),
        }
    }
}

#[derive(PartialEq, Clone, Default)]
pub struct ProjectState {
    pub audio_filename: Option<String>,
    pub lrc_filename: Option<String>,
}

#[derive(PartialEq, Clone, Default)]
pub struct AppState {
    pub playback: PlaybackState,
    pub document: DocumentState,
    pub history: HistoryState,
    pub view: ViewState,
    pub project: ProjectState,
}

impl AppState {
    pub fn max_timeline_duration(&self) -> TimeMs {
        let last_lyric_ms = self.document.document.as_ref()
            .and_then(|doc| doc.last_entry_time_ms())
            .unwrap_or(TimeMs(0));
        TimeMs(self.playback.duration_ms.as_u32().max(last_lyric_ms.as_u32()) + 15000)
    }

    pub fn trigger_lrc_export(&self) {
        let text = self.document.source_text.clone();
        if let Some(window) = web_sys::window()
            && let Some(document) = window.document()
        {
            let filename = if let Some(audio_name) = &self.project.audio_filename {
                let base = audio_name.rfind('.').map(|i| &audio_name[..i]).unwrap_or(audio_name);
                format!("{}.lrc", base)
            } else if let Some(lrc_name) = &self.project.lrc_filename {
                lrc_name.clone()
            } else {
                "lyrics.lrc".to_string()
            };

            if let Ok(blob) = web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&text.into()))
                && let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob)
                && let Ok(a) = document.create_element("a")
                && let Ok(a) = a.dyn_into::<web_sys::HtmlAnchorElement>()
            {
                a.set_href(&url);
                a.set_download(&filename);
                a.click();
                let _ = web_sys::Url::revoke_object_url(&url);
            }
        }
    }

    pub fn update_document(&mut self, source: String) {
        self.document.source_text = source.clone();
        let parser = LrcParser::new(&source);
        match parser.parse() {
            Ok(mut doc) => {
                doc.reconcile_identity(
                    self.document.document.as_ref(),
                    &mut self.document.next_uid,
                );
                self.view.selection.prune(&doc);
                self.document.document = Some(doc);
                self.document.parse_error = None;
            }
            Err(e) => {
                self.document.parse_error = Some(e.prefixed_message());
            }
        }
    }
}
