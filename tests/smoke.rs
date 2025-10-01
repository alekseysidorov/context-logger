use crate::common::{RecordExt, check_logger_once};

pub mod common;

#[test]
fn test_smoke() {
    check_logger_once(|entry| {
        let val = entry.get_record("answer").unwrap();
        assert_eq!(val, 42);
        Ok(())
    });

    let _guard = context_logger::LogContext::new()
        .record("answer", 42)
        .enter();
    log::info!("Smoke on the water, fire in the sky");
}
