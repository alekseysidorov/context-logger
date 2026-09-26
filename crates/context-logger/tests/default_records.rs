// Warning: Because each test initializes the logger, we need to split the
// tests into separate files to avoid multiple initializations of the logger.

use context_logger::{ContextLogger, LogValue};
use serde_json::json;

use crate::common::channel_logger;

pub mod common;

#[test]
fn test_default() {
    let (logger, records) = channel_logger();
    ContextLogger::new(logger)
        .with_default_field("tag", 42)
        .with_default_field_fn("my_log_level", |log_record| log_record.level().to_string())
        .with_default_field_fn("thread_name", |_| {
            LogValue::serde(std::thread::current().name().map(ToOwned::to_owned))
        })
        .init(log::LevelFilter::Trace);

    log::info!("Wazzup everyone!");

    let record = records.recv().unwrap();
    assert_eq!(
        record.fields,
        json!({
            "tag": 42,
            "my_log_level": "INFO",
            "thread_name": "test_default",
        })
        .as_object()
        .unwrap()
        .clone()
    );
    assert_eq!(record.message, "Wazzup everyone!");
}
