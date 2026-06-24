// Warning: Because each test initializes the logger, we need to split the
// tests into separate files to avoid multiple initializations of the logger.

use context_logger::{LogContext, LogContextExt};

use crate::common::{RecordExt, check_logger_once};

pub mod common;

#[test]
fn test_inherited_records_shadowing() {
    check_logger_once(|entry| {
        assert_eq!(entry.get_record("answer").unwrap(), 42);
        assert_eq!(entry.get_record("name").unwrap(), "Robin");
        assert_eq!(entry.get_record("shadow").unwrap(), true);
        assert_eq!(entry.get_record("inherited_shadow").unwrap(), "child");
        Ok(())
    });

    LogContext::new()
        .with_inherited_record("answer", 42)
        .with_inherited_record("shadow", false)
        .with_inherited_record("inherited_shadow", "parent")
        .in_scope(|| {
            LogContext::new()
                .with_inherited_record("inherited_shadow", "child")
                .with_local_record("name", "Robin")
                .with_local_record("shadow", true)
                .in_scope(|| {
                    log::info!("Ipsum dolor sit amet, consectetur adipiscing elit");
                });
        });
}
