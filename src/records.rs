use std::{borrow::Cow, collections::HashMap};

use crate::LogValue;

pub type LogRecordsIter<'a> = std::collections::hash_map::Iter<'a, Cow<'static, str>, LogValue>;
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
    /// Creates a new, empty set of records.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a key-value record into this collection, returning the collection for chained calls.
    ///
    /// This method takes ownership of `self`, so it can be used as part of a
    /// builder-style chain:
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogRecords;
    ///
    /// let records = LogRecords::new()
    ///       .with_record("user_id", "user-123")
    ///       .with_record("request_id", 42);
    /// ```
    #[must_use]
    pub fn with_record(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<LogValue>,
    ) -> Self {
        self.insert(key, value);
        self
    }

    /// Inserts a key-value record into this collection.
    ///
    /// Unlike [`with_record`](LogRecords::with_record), this method borrows `self` and
    /// returns a mutable reference, allowing it to be used when chaining with other methods
    /// that require borrowing.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogRecords;
    ///
    /// let mut records = LogRecords::new();
    /// records
    ///     .insert("user_id", "user-123")
    ///     .insert("request_id", 42);
    /// ```
    pub fn insert(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<LogValue>,
    ) -> &mut Self {
        self.0.insert(key.into(), value.into());
        self
    }

    /// Merges this collection with the records from another collection.
    ///
    /// This method borrows `self` and returns a mutable reference, allowing it
    /// to be used when chaining with other methods that require borrowing.
    ///
    /// # Merging policy
    ///
    /// Keys in this collection with duplicate names will be overwritten by keys from the
    /// provided collection. The order of keys in the resulting collection is undefined.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogRecords;
    ///
    /// # let other_records = LogRecords::new();
    /// let mut records = LogRecords::new();
    /// records
    ///     .insert("user_id", "Alice")
    ///     .merge_with(other_records)
    ///     .insert("request_id", 42);
    /// ```
    pub fn merge_with(&mut self, other: impl IntoIterator<Item = LogRecord>) -> &mut Self {
        self.0.extend(other);
        self
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

impl Extend<LogRecord> for LogRecords {
    fn extend<I: IntoIterator<Item = LogRecord>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl FromIterator<LogRecord> for LogRecords {
    fn from_iter<T: IntoIterator<Item = LogRecord>>(iter: T) -> Self {
        Self(HashMap::from_iter(iter))
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
