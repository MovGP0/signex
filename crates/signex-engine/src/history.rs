use signex_types::schematic::SchematicSheet;

use crate::EngineError;
use crate::patch::PatchPair;

use super::Engine;

pub const MAX_HISTORY_ENTRIES: usize = 100;

#[derive(Debug)]
pub struct HistoryEntry {
    pub snapshot: SchematicSheet,
    pub patch_pair: PatchPair,
}

impl Engine {
    #[must_use]
    pub const fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    #[must_use]
    pub const fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Restores the preceding document snapshot.
    ///
    /// # Errors
    ///
    /// Returns an engine error if history restoration fails.
    pub fn undo(&mut self) -> Result<Option<PatchPair>, EngineError> {
        let Some(entry) = self.history.pop() else {
            return Ok(None);
        };

        let patch_pair = entry.patch_pair;
        let redo_snapshot = std::mem::replace(&mut self.document, entry.snapshot);
        self.redo_stack.push(HistoryEntry {
            snapshot: redo_snapshot,
            patch_pair,
        });
        Ok(Some(patch_pair))
    }

    /// Restores the next document snapshot.
    ///
    /// # Errors
    ///
    /// Returns an engine error if history restoration fails.
    pub fn redo(&mut self) -> Result<Option<PatchPair>, EngineError> {
        let Some(entry) = self.redo_stack.pop() else {
            return Ok(None);
        };

        let patch_pair = entry.patch_pair;
        let undo_snapshot = std::mem::replace(&mut self.document, entry.snapshot);
        self.history.push(HistoryEntry {
            snapshot: undo_snapshot,
            patch_pair,
        });
        Ok(Some(patch_pair))
    }

    pub(crate) fn record_history(&mut self, snapshot: SchematicSheet, patch_pair: PatchPair) {
        if self.history.len() >= MAX_HISTORY_ENTRIES {
            self.history.remove(0);
        }
        self.history.push(HistoryEntry {
            snapshot,
            patch_pair,
        });
        self.redo_stack.clear();
    }
}
