use context_logger::ContextLogger;
use log::{LevelFilter, Record, kv::Key};

pub trait RecordExt {
    fn get_record(&self, key: &str) -> Option<serde_json::Value>;
}

impl RecordExt for Record<'_> {
    fn get_record(&self, key: &str) -> Option<serde_json::Value> {
        let key = Key::from_str(key);
        let val = self.key_values().get(key)?;
        serde_json::to_value(val).ok()
    }
}

pub fn check_logger_once<F>(check: F)
where
    F: Fn(&Record) -> std::io::Result<()> + Send + Sync + 'static,
{
    let level_filter = LevelFilter::Trace;
    let logger = ContextLogger::new(
        env_logger::Builder::new()
            .filter_level(level_filter)
            .format(move |_fmt, record| check(record))
            .build(),
    );
    logger.init(level_filter);
}
