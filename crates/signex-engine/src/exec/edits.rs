//! `Engine::exec_edits` — see `exec/mod.rs`.

use crate::{
    Command, CommandResult, DocumentPatch, Engine, EngineError, PatchPair, SchematicSheet,
    SemanticPatch, TextTarget, command,
};

impl Engine {
    #[expect(
        clippy::too_many_lines,
        clippy::useless_let_if_seq,
        reason = "command dispatch keeps edit variants and their patch recording together"
    )]
    pub(crate) fn exec_edits(
        &mut self,
        before: SchematicSheet,
        cmd: Command,
    ) -> Result<CommandResult, EngineError> {
        match cmd {
            Command::ReplaceDocument { document } => {
                self.document = document;

                let patch_pair = PatchPair {
                    semantic: SemanticPatch::DocumentReplaced,
                    document: DocumentPatch::FULL,
                };

                self.record_history(before, patch_pair);

                Ok(CommandResult::changed(patch_pair))
            }
            Command::UpdateText { target, value } => {
                let document_patch = match target {
                    TextTarget::Label(_) => DocumentPatch::LABELS,
                    TextTarget::TextNote(_) => DocumentPatch::TEXT_NOTES,
                    TextTarget::SymbolReference(_) | TextTarget::SymbolValue(_) => {
                        DocumentPatch::SYMBOLS
                    }
                };

                let changed = match target {
                    TextTarget::Label(uuid) => self
                        .document
                        .labels
                        .iter_mut()
                        .find(|label| label.uuid == uuid)
                        .is_some_and(|label| {
                            if label.text == value {
                                false
                            } else {
                                label.text = value;
                                true
                            }
                        }),
                    TextTarget::TextNote(uuid) => self
                        .document
                        .text_notes
                        .iter_mut()
                        .find(|text_note| text_note.uuid == uuid)
                        .is_some_and(|text_note| {
                            if text_note.text == value {
                                false
                            } else {
                                text_note.text = value;
                                true
                            }
                        }),
                    TextTarget::SymbolReference(uuid) => self
                        .document
                        .symbols
                        .iter_mut()
                        .find(|symbol| symbol.uuid == uuid)
                        .is_some_and(|symbol| {
                            if symbol.reference == value {
                                false
                            } else {
                                symbol.reference = value;
                                true
                            }
                        }),
                    TextTarget::SymbolValue(uuid) => self
                        .document
                        .symbols
                        .iter_mut()
                        .find(|symbol| symbol.uuid == uuid)
                        .is_some_and(|symbol| {
                            if symbol.value == value {
                                false
                            } else {
                                symbol.value = value;
                                true
                            }
                        }),
                };

                if !changed {
                    return Ok(CommandResult::unchanged());
                }

                let patch_pair = PatchPair {
                    semantic: SemanticPatch::TextUpdated,
                    document: document_patch,
                };

                self.record_history(before, patch_pair);

                Ok(CommandResult::changed(patch_pair))
            }
            Command::UpdateLabelProps {
                label_id,
                font_size_mm,
                justify,
                rotation_degrees,
            } => {
                let changed = self
                    .document
                    .labels
                    .iter_mut()
                    .find(|l| l.uuid == label_id)
                    .is_some_and(|l| {
                        let mut any = false;
                        if let Some(fs) = font_size_mm
                            && (l.font_size - fs).abs() > 1e-6
                        {
                            l.font_size = fs;
                            any = true;
                        }
                        if let Some(j) = justify
                            && l.justify != j
                        {
                            l.justify = j;
                            any = true;
                        }
                        if let Some(r) = rotation_degrees
                            && (l.rotation - r).abs() > 1e-6
                        {
                            l.rotation = r;
                            any = true;
                        }
                        any
                    });

                if !changed {
                    return Ok(CommandResult::unchanged());
                }

                let patch_pair = PatchPair {
                    semantic: SemanticPatch::LabelsMutated,
                    document: DocumentPatch::LABELS,
                };

                self.record_history(before, patch_pair);

                Ok(CommandResult::changed(patch_pair))
            }
            Command::SetSymbolRotation {
                symbol_id,
                rotation_degrees,
            } => {
                let changed = self
                    .document
                    .symbols
                    .iter_mut()
                    .find(|s| s.uuid == symbol_id)
                    .is_some_and(|s| {
                        if (s.rotation - rotation_degrees).abs() < 1e-6 {
                            false
                        } else {
                            s.rotation = rotation_degrees;
                            true
                        }
                    });

                if !changed {
                    return Ok(CommandResult::unchanged());
                }

                let patch_pair = PatchPair {
                    semantic: SemanticPatch::SelectionRotated,
                    document: DocumentPatch::SYMBOLS,
                };

                self.record_history(before, patch_pair);

                Ok(CommandResult::changed(patch_pair))
            }
            Command::UpdateSymbolTextSize {
                symbol_id,
                field,
                font_size_mm,
            } => {
                use command::SymbolTextField;
                let changed = self
                    .document
                    .symbols
                    .iter_mut()
                    .find(|s| s.uuid == symbol_id)
                    .is_some_and(|s| {
                        let tp = match field {
                            SymbolTextField::Reference => s.ref_text.as_mut(),
                            SymbolTextField::Value => s.val_text.as_mut(),
                        };
                        if let Some(tp) = tp
                            && (tp.font_size - font_size_mm).abs() > 1e-6
                        {
                            tp.font_size = font_size_mm;
                            true
                        } else {
                            false
                        }
                    });

                if !changed {
                    return Ok(CommandResult::unchanged());
                }

                let patch_pair = PatchPair {
                    semantic: SemanticPatch::SymbolFieldsUpdated,
                    document: DocumentPatch::SYMBOLS,
                };

                self.record_history(before, patch_pair);

                Ok(CommandResult::changed(patch_pair))
            }
            Command::UpdateSymbolLibId { symbol_id, lib_id } => {
                let changed = self
                    .document
                    .symbols
                    .iter_mut()
                    .find(|s| s.uuid == symbol_id)
                    .is_some_and(|s| {
                        if s.lib_id == lib_id {
                            false
                        } else {
                            s.lib_id = lib_id;
                            true
                        }
                    });

                if !changed {
                    return Ok(CommandResult::unchanged());
                }

                let patch_pair = PatchPair {
                    semantic: SemanticPatch::SymbolFieldsUpdated,
                    document: DocumentPatch::SYMBOLS,
                };

                self.record_history(before, patch_pair);

                Ok(CommandResult::changed(patch_pair))
            }
            Command::UpdateSymbolFootprint {
                symbol_id,
                footprint,
            } => {
                let changed = self
                    .document
                    .symbols
                    .iter_mut()
                    .find(|symbol| symbol.uuid == symbol_id)
                    .is_some_and(|symbol| {
                        if symbol.footprint == footprint {
                            false
                        } else {
                            symbol.footprint = footprint;
                            true
                        }
                    });

                if !changed {
                    return Ok(CommandResult::unchanged());
                }

                let patch_pair = PatchPair {
                    semantic: SemanticPatch::SymbolFieldsUpdated,
                    document: DocumentPatch::SYMBOLS,
                };

                self.record_history(before, patch_pair);

                Ok(CommandResult::changed(patch_pair))
            }
            Command::SetSymbolField {
                symbol_id,
                key,
                value,
            } => {
                // Reject overwrites of reserved keys — those have their
                // own commands so ref/value/footprint moves stay tracked.
                match key.to_lowercase().as_str() {
                    "reference" | "value" | "footprint" => {
                        return Ok(CommandResult::unchanged());
                    }
                    _ => {}
                }
                let changed = self
                    .document
                    .symbols
                    .iter_mut()
                    .find(|s| s.uuid == symbol_id)
                    .is_some_and(|symbol| {
                        if value.is_empty() {
                            symbol.fields.remove(&key).is_some()
                        } else if symbol.fields.get(&key) == Some(&value) {
                            false
                        } else {
                            symbol.fields.insert(key, value);
                            true
                        }
                    });
                if !changed {
                    return Ok(CommandResult::unchanged());
                }
                let patch_pair = PatchPair {
                    semantic: SemanticPatch::SymbolFieldsUpdated,
                    document: DocumentPatch::SYMBOLS,
                };
                self.record_history(before, patch_pair);
                Ok(CommandResult::changed(patch_pair))
            }
            Command::UpdateSymbolFields {
                symbol_id,
                reference,
                value,
                footprint,
            } => {
                let changed = self
                    .document
                    .symbols
                    .iter_mut()
                    .find(|symbol| symbol.uuid == symbol_id)
                    .is_some_and(|symbol| {
                        let changed = symbol.reference != reference
                            || symbol.value != value
                            || symbol.footprint != footprint;

                        if changed {
                            symbol.reference = reference;
                            symbol.value = value;
                            symbol.footprint = footprint;
                        }

                        changed
                    });

                if !changed {
                    return Ok(CommandResult::unchanged());
                }

                let patch_pair = PatchPair {
                    semantic: SemanticPatch::SymbolFieldsUpdated,
                    document: DocumentPatch::SYMBOLS,
                };

                self.record_history(before, patch_pair);

                Ok(CommandResult::changed(patch_pair))
            }
            _ => unreachable!(),
        }
    }
}
