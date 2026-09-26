// Warning: Because each test initializes the logger, we need to split the
// tests into separate files to avoid multiple initializations of the logger.

use context_logger::{LogContext, LogContextExt};

use crate::common::{LogRecordExt as _, check_logger_once};

pub mod common;

#[test]
fn test_inherited_fields_shadowing() {
    check_logger_once(
        |logger| logger,
        |record| {
            assert_eq!(record.get_field("answer").unwrap(), 42);
            assert_eq!(record.get_field("name").unwrap(), "Robin");
            assert_eq!(record.get_field("shadow").unwrap(), true);
            assert_eq!(record.get_field("inherited_shadow").unwrap(), "child");
            Ok(())
        },
    );

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
}
