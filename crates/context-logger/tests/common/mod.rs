use std::sync::mpsc::{self, Receiver, Sender};

use log::{Level, LevelFilter, Log, Metadata, Record, kv};
use serde_json::{Map, Value};

#[derive(Debug, Eq, PartialEq)]
pub struct RecordSnapshot {
    pub level: Level,
    pub target: String,
    pub message: String,
    pub fields: Map<String, Value>,
}

#[must_use]
pub fn init_channel_logger() -> (ChannelLogger, Receiver<RecordSnapshot>) {
    let (sender, receiver) = mpsc::channel();
    (ChannelLogger { sender }, receiver)
}

#[derive(Debug)]
pub struct ChannelLogger {
    sender: Sender<RecordSnapshot>,
}

impl Log for ChannelLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= LevelFilter::Trace
    }

    fn log(&self, record: &Record<'_>) {
        self.sender
            .send(RecordSnapshot::from_record(record))
            .expect("test logger receiver was dropped");
    }

    fn flush(&self) {}
}

impl RecordSnapshot {
    fn from_record(record: &Record<'_>) -> Self {
        let mut fields = Map::new();
        record
            .key_values()
            .visit(&mut JsonFields(&mut fields))
            .expect("test logger failed to visit record fields");

        Self {
            level: record.level(),
            target: record.target().to_owned(),
            message: record.args().to_string(),
            fields,
        }
    }
}

struct JsonFields<'a>(&'a mut Map<String, Value>);

impl<'kvs> kv::VisitSource<'kvs> for JsonFields<'_> {
    fn visit_pair(&mut self, key: kv::Key<'kvs>, value: kv::Value<'kvs>) -> Result<(), kv::Error> {
        let value = serde_json::to_value(value).expect("log field is not JSON serializable");
        self.0.insert(key.to_string(), value);
        Ok(())
    }
}
