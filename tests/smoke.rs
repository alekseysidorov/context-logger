use context_logger::{LogContext, LogScope};

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
