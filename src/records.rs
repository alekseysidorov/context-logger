use std::{borrow::Cow, collections::HashMap};

use crate::LogValue;

/// Iterator over log records in a [`LogRecords`] collection.
pub type LogRecordsIter<'a> = std::collections::hash_map::Iter<'a, Cow<'static, str>, LogValue>;
/// Iterator over log records in a [`LogRecords`] collection that takes ownership of the collection.
pub type LogRecordsIntoIter = std::collections::hash_map::IntoIter<Cow<'static, str>, LogValue>;

pub type LogRecord = (Cow<'static, str>, LogValue);
pub type LogRecordRef<'a> = (&'a Cow<'static, str>, &'a LogValue);

/// A set of records that can be attached to a logging scope.
///
/// [`LogRecords`] represents a set of key-value pairs that can be
/// added to log messages when the log context scope is active.
///
/// # Ordering
///
/// The order in which records appear is **not guaranteed**. Do not rely on any specific
/// ordering of keys.
#[derive(Debug, Clone, Default)]
pub struct LogRecords(pub(crate) HashMap<Cow<'static, str>, LogValue>);

impl LogRecords {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a key-value record to this collection, returning the collection for chained calls.
    ///
    /// # Examples
    ///
    /// This method takes ownership of `self`, so it can be used as part of a
    /// builder-style chain:
    ///
    /// ```
    /// use context_logger::LogRecords;
    ///
    /// let records = LogRecords::new()
    ///     .field("user_id", "user-123")
    ///     .field("request_id", 42);
    /// ```
    #[must_use]
    pub fn field(mut self, key: impl Into<Cow<'static, str>>, value: impl Into<LogValue>) -> Self {
        self.insert(key, value);
        self
    }

    /// Adds a key-value record to this collection.
    pub fn insert(&mut self, key: impl Into<Cow<'static, str>>, value: impl Into<LogValue>) {
        self.0.insert(key.into(), value.into());
    }

    /// Extends this collection with the records from another collection.
    pub fn extend(&mut self, other: impl IntoIterator<Item = LogRecord>) {
        self.0.extend(other);
    }

    /// Returns an iterator over the records in this collection.
    #[must_use]
    pub fn iter(&self) -> LogRecordsIter<'_> {
        self.0.iter()
    }

    /// Returns `true` if this collection contains no records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a LogRecords {
    type Item = LogRecordRef<'a>;
    type IntoIter = LogRecordsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for LogRecords {
    type Item = LogRecord;
    type IntoIter = LogRecordsIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
impl LogRecords {
    /// Returns a reference to the value associated with the given key, if it exists.
    pub(crate) fn find(&self, key: impl AsRef<str>) -> Option<&LogValue> {
        self.0.get(&Cow::Owned(key.as_ref().to_owned()))
    }
}

#[cfg(test)]
impl std::ops::Index<&str> for LogRecords {
    type Output = LogValue;

    fn index(&self, index: &str) -> &Self::Output {
        self.0
            .get(&Cow::Owned(index.to_owned()))
            .expect("No record found for the given key")
    }
}
