//! Pure conflict analysis and immutable keymap compilation.
mod compiler;
mod conflicts;
pub use compiler::{CompileError, CompiledEntry, CompiledKeymap, compile};
pub use conflicts::{Conflict, ConflictCode, ConflictSeverity, analyze, analyze_candidate};
