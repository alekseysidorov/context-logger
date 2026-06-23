use context_logger::{LogContext, LogContextExt, LogScope};

use crate::common::{RecordExt, check_logger_once};

pub mod common;

#[test]
fn test_smoke() {
    check_logger_once(|entry| {
        let val = entry.get_record("answer").unwrap();
        assert_eq!(val, 42);
        Ok(())
    });

    let _guard = LogScope::enter(LogContext::new().local_record("answer", 42));
    log::info!("Smoke on the water, fire in the sky");
}

#[test]
fn test_inherited_records_shadowing() {
    check_logger_once(|entry| {
        assert_eq!(entry.get_record("answer").unwrap(), 42);
        assert_eq!(entry.get_record("name").unwrap(), "Robin");
        assert_eq!(entry.get_record("shadow").unwrap(), true);
        Ok(())
    });

    LogContext::new()
        .inherited_record("answer", 42)
        .inherited_record("shadow", false)
        .in_scope(|| {
            LogContext::new()
                .local_record("name", "Robin")
                .local_record("shadow", true)
                .in_scope(|| {
                    log::info!("Ipsum dolor sit amet, consectetur adipiscing elit");
                });
        });
}
