#![deny(unsafe_code)]

use std::collections::VecDeque;
use std::time::SystemTime;

/// The severity assigned to a diagnostics event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Info,
    Warning,
    Error,
}

/// Bounded, structured metadata about an application event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub timestamp: SystemTime,
    pub level: DiagnosticLevel,
    pub component: &'static str,
    pub event: &'static str,
    pub error_code: Option<&'static str>,
}

/// An in-memory, per-instance bounded recorder for diagnostic events.
#[derive(Debug)]
pub struct DiagnosticsRecorder {
    events: VecDeque<DiagnosticEvent>,
    capacity: usize,
}

impl DiagnosticsRecorder {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn record(&mut self, event: DiagnosticEvent) {
        if self.capacity == 0 {
            return;
        }

        while self.events.len() >= self.capacity {
            self.events.pop_front();
        }

        self.events.push_back(event);
    }

    pub fn events(&self) -> impl Iterator<Item = &DiagnosticEvent> {
        self.events.iter()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticEvent, DiagnosticLevel, DiagnosticsRecorder};
    use std::time::SystemTime;

    fn event(name: &'static str) -> DiagnosticEvent {
        DiagnosticEvent {
            timestamp: SystemTime::UNIX_EPOCH,
            level: DiagnosticLevel::Info,
            component: "diagnostics",
            event: name,
            error_code: None,
        }
    }

    #[test]
    fn capacity_is_bounded_and_evicts_oldest_events() {
        let mut recorder = DiagnosticsRecorder::new(2);
        recorder.record(event("first"));
        recorder.record(event("second"));
        recorder.record(event("third"));

        assert_eq!(recorder.len(), 2);
        assert_eq!(
            recorder
                .events()
                .map(|event| event.event)
                .collect::<Vec<_>>(),
            ["second", "third"]
        );
    }

    #[test]
    fn zero_capacity_recorder_discards_events() {
        let mut recorder = DiagnosticsRecorder::new(0);
        recorder.record(event("discarded"));

        assert!(recorder.is_empty());
        assert_eq!(recorder.len(), 0);
    }

    #[test]
    fn preserves_safe_structured_metadata() {
        let mut recorder = DiagnosticsRecorder::new(1);
        let expected = DiagnosticEvent {
            timestamp: SystemTime::UNIX_EPOCH,
            level: DiagnosticLevel::Error,
            component: "sync",
            event: "request_failed",
            error_code: Some("SYNC_REQUEST_FAILED"),
        };

        recorder.record(expected.clone());

        assert_eq!(recorder.events().next(), Some(&expected));
    }
}
