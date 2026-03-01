use std::sync::{Arc, Mutex};

use context_logger::LogContext;

pub mod common;
use crate::common::{RecordExt, check_logger_once};

/// Collects key-value pairs from successive log calls for assertion.
#[derive(Debug, Default)]
struct LogCapture {
    entries: Vec<std::collections::HashMap<String, serde_json::Value>>,
}

impl LogCapture {
    fn push(&mut self, record: &log::Record<'_>, keys: &[&str]) {
        let mut map = std::collections::HashMap::new();
        for &key in keys {
            if let Some(val) = record.get_record(key) {
                map.insert(key.to_owned(), val);
            }
        }
        self.entries.push(map);
    }
}

/// Verifies that inherited records are visible in nested contexts while local
/// records are only visible in the context they were added to.
#[test]
fn test_inherited_vs_local_records() {
    let capture: Arc<Mutex<LogCapture>> = Arc::default();
    let capture_clone = capture.clone();

    check_logger_once(move |record| {
        capture_clone
            .lock()
            .unwrap()
            .push(record, &["request_id", "handler", "step"]);
        Ok(())
    });

    // Outer context: request_id is inherited (should propagate), handler is local (should not).
    let _outer = LogContext::new()
        .inherited_record("request_id", "req-123")
        .record("handler", "outer_handler")
        .enter();

    // Inner context: step is local.
    {
        let _inner = LogContext::new().record("step", "inner_step").enter();

        // Inside inner context: inherited request_id is visible, local handler is NOT.
        log::info!("inside inner context");
    }

    // Back in outer context: all outer records are visible again.
    log::info!("back in outer context");

    let cap = capture.lock().unwrap();
    assert_eq!(cap.entries.len(), 2, "expected two log entries");

    // First entry: logged from inside the inner context.
    let inner_entry = &cap.entries[0];
    assert_eq!(
        inner_entry.get("request_id").and_then(|v| v.as_str()),
        Some("req-123"),
        "inherited record must be visible inside nested context"
    );
    assert!(
        inner_entry.get("step").is_some(),
        "inner local record must be visible inside its own context"
    );
    assert!(
        inner_entry.get("handler").is_none(),
        "outer local record must NOT be visible inside nested context"
    );

    // Second entry: logged from the outer context after inner guard was dropped.
    let outer_entry = &cap.entries[1];
    assert_eq!(
        outer_entry.get("request_id").and_then(|v| v.as_str()),
        Some("req-123"),
        "inherited record must remain visible in the context that defined it"
    );
    assert_eq!(
        outer_entry.get("handler").and_then(|v| v.as_str()),
        Some("outer_handler"),
        "outer local record must be visible when its context is active"
    );
    assert!(
        outer_entry.get("step").is_none(),
        "inner local record must NOT be visible after inner context is dropped"
    );
}
