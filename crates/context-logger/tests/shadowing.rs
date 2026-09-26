// Warning: Because each test initializes the logger, we need to split the
// tests into separate files to avoid multiple initializations of the logger.

use context_logger::{ContextLogger, LogContext, LogContextExt};
use serde_json::json;

use crate::common::channel_logger;

pub mod common;

#[test]
fn test_inherited_fields_shadowing() {
    let (logger, records) = channel_logger();
    ContextLogger::new(logger).init(log::LevelFilter::Trace);

    LogContext::new()
        .with_inherited_field("answer", 42)
        .with_inherited_field("shadow", false)
        .with_inherited_field("inherited_shadow", "parent")
        .in_scope(|| {
            LogContext::new()
                .with_inherited_field("inherited_shadow", "child")
                .with_local_field("name", "Robin")
                .with_local_field("shadow", true)
                .in_scope(|| {
                    log::info!("Ipsum dolor sit amet, consectetur adipiscing elit");
                });
        });

    let record = records.recv().unwrap();
    assert_eq!(
        record.fields,
        json!({
            "answer": 42,
            "name": "Robin",
            "shadow": true,
            "inherited_shadow": "child",
        })
        .as_object()
        .unwrap()
        .clone()
    );
}
