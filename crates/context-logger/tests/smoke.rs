// Warning: Because each test initializes the logger, we need to split the
// tests into separate files to avoid multiple initializations of the logger.

use context_logger::{ContextLogger, LogContext, LogScope};
use serde_json::json;

use crate::common::init_channel_logger;

pub mod common;

#[test]
fn test_smoke() {
    let (logger, records) = init_channel_logger();
    ContextLogger::new(logger).init(log::LevelFilter::Trace);

    let _guard = LogScope::enter(LogContext::new().with_local_field("answer", 42));
    log::info!("Smoke on the water, fire in the sky");

    assert_eq!(
        records.recv().unwrap().fields,
        json!({ "answer": 42 }).as_object().unwrap().clone()
    );
}
