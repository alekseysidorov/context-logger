// Warning: Because each test initializes the logger, we need to split the
// tests into separate files to avoid multiple initializations of the logger.

use context_logger::LogValue;
use pretty_assertions::assert_eq;

use crate::common::{RecordExt, check_logger_once};

pub mod common;

#[test]
fn test_default() {
    check_logger_once(
        |logger| {
            logger
                .with_default_record("tag", 42)
                .with_default_record_fn("my_log_level", |log_record| log_record.level().to_string())
                .with_default_record_fn("thread_name", |_| {
                    LogValue::serde(std::thread::current().name().map(ToOwned::to_owned))
                })
        },
        |entry| {
            assert_eq!(entry.get_record("tag").unwrap(), 42);
            assert_eq!(entry.get_record("my_log_level").unwrap(), "INFO");
            assert_eq!(entry.get_record("thread_name").unwrap(), "test_default");
            Ok(())
        },
    );

    log::info!("Wazzup everyone!");
}
