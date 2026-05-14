//! Internal representation of a single contextual log record.
//!
//! This module provides the [`LogRecord`] type, which is used internally by the
//! context-logger library to represent individual context fields before they are
//! converted to the appropriate format for the underlying logging implementation.

use std::borrow::Cow;

use crate::LogValue;

/// A single contextual log record.
///
/// This type represents a key-value pair that can be attached to log records
/// through the [`crate::LogContext`] API. It is used internally by the
/// context-logger library.
#[derive(Debug, Clone)]
pub struct LogRecord {
    key: Cow<'static, str>,
    value: LogValue,
}

impl LogRecord {
    /// Creates a new [`LogRecord`] with the given key and value.
    pub const fn new(key: Cow<'static, str>, value: LogValue) -> Self {
        Self { key, value }
    }

    /// Returns the key of this record.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the value of this record.
    pub const fn value(&self) -> &LogValue {
        &self.value
    }
}

impl<K, V> From<(K, V)> for LogRecord
where
    K: Into<Cow<'static, str>>,
    V: Into<LogValue>,
{
    fn from((key, value): (K, V)) -> Self {
        Self::new(key.into(), value.into())
    }
}
