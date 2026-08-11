use super::{Conflict, ConflictSeverity, analyze};
use crate::domain::{Action, Chord, Mapping};
use std::collections::HashMap;
use uuid::Uuid;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledEntry {
    pub mapping_id: Uuid,
    pub action: Action,
}
#[derive(Clone, Debug)]
pub struct CompiledKeymap {
    entries: HashMap<Chord, CompiledEntry>,
    warnings: Vec<Conflict>,
}
impl CompiledKeymap {
    pub fn lookup(&self, chord: &Chord) -> Option<&CompiledEntry> {
        self.entries.get(chord)
    }
    pub fn warnings(&self) -> &[Conflict] {
        &self.warnings
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileError {
    pub conflicts: Vec<Conflict>,
}
pub fn compile(mappings: &[Mapping]) -> Result<CompiledKeymap, CompileError> {
    let conflicts = analyze(mappings);
    let blocking: Vec<_> = conflicts
        .iter()
        .filter(|conflict| conflict.severity == ConflictSeverity::Error)
        .cloned()
        .collect();
    if !blocking.is_empty() {
        return Err(CompileError {
            conflicts: blocking,
        });
    }
    let entries = mappings
        .iter()
        .filter(|mapping| mapping.enabled)
        .map(|mapping| {
            (
                mapping.trigger.chord().clone(),
                CompiledEntry {
                    mapping_id: mapping.id,
                    action: mapping.action.clone(),
                },
            )
        })
        .collect();
    Ok(CompiledKeymap {
        entries,
        warnings: conflicts,
    })
}
