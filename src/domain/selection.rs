use crate::domain::document::LrcDocument;
use crate::domain::entry::LyricEntry;

/// How a chunk click should update selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionMode {
    /// Replace selection with the clicked chunk.
    Replace,
    /// Toggle the clicked chunk.
    Toggle,
    /// Expand from the anchor to the clicked chunk.
    Range,
}

/// Selected lyric chunk ids and range anchor.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectionState {
    selected_ids: Vec<usize>,
    anchor_id: Option<usize>,
}

impl SelectionState {
    /// Current selected ids.
    pub fn selected_ids(&self) -> &[usize] {
        &self.selected_ids
    }

    /// Anchor used for range selection.
    pub fn anchor_id(&self) -> Option<usize> {
        self.anchor_id
    }

    /// Whether an id is currently selected.
    pub fn contains(&self, id: usize) -> bool {
        self.selected_ids.contains(&id)
    }

    /// Keep only selections that still exist in the document.
    pub fn prune(&mut self, document: &LrcDocument) {
        self.selected_ids
            .retain(|id| document.entry_by_id(*id).is_some());

        if self
            .anchor_id
            .is_some_and(|id| document.entry_by_id(id).is_none())
        {
            self.anchor_id = None;
        }
    }

    /// Replace with the active lyric unless multiple chunks are selected.
    pub fn sync_to_active(&mut self, entry: Option<&LyricEntry>, preserve_selection: bool) {
        if preserve_selection {
            return;
        }

        self.selected_ids.clear();

        if let Some(entry) = entry.filter(|entry| !entry.is_empty()) {
            self.selected_ids.push(entry.id());
            self.anchor_id = Some(entry.id());
        }
    }

    /// Select every non-empty entry.
    pub fn select_all(&mut self, document: &LrcDocument) {
        self.selected_ids = document
            .entries()
            .iter()
            .filter(|entry| !entry.is_empty())
            .map(LyricEntry::id)
            .collect();
        self.anchor_id = self.selected_ids.first().copied();
    }

    /// Apply a click selection transition.
    pub fn select_entry(&mut self, document: &LrcDocument, entry_id: usize, mode: SelectionMode) {
        let Some(entry) = document.entry_by_id(entry_id) else {
            return;
        };

        match mode {
            SelectionMode::Replace => {
                self.selected_ids.clear();
                if !entry.is_empty() {
                    self.selected_ids.push(entry_id);
                }
                self.anchor_id = Some(entry_id);
            }
            SelectionMode::Toggle => {
                if let Some(index) = self.selected_ids.iter().position(|id| *id == entry_id) {
                    self.selected_ids.remove(index);
                } else if !entry.is_empty() {
                    self.selected_ids.push(entry_id);
                    self.selected_ids.sort_unstable();
                }
                self.anchor_id = Some(entry_id);
            }
            SelectionMode::Range => {
                let anchor_id = self.anchor_id.unwrap_or(entry_id);
                let start = anchor_id.min(entry_id);
                let end = anchor_id.max(entry_id);
                self.selected_ids = document
                    .entries()
                    .iter()
                    .filter(|entry| {
                        !entry.is_empty() && (start..=end).contains(&entry.id())
                    })
                    .map(LyricEntry::id)
                    .collect();
            }
        }
    }

    /// True when source text selection should be suppressed.
    pub fn suppresses_source_selection(&self) -> bool {
        self.selected_ids.len() > 1
    }
}
