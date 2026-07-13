use std::{borrow::Cow, collections::HashMap};

use crate::LogValue;

pub type LogFieldsIter<'a> = std::collections::hash_map::Iter<'a, Cow<'static, str>, LogValue>;
pub type LogFieldsIntoIter = std::collections::hash_map::IntoIter<Cow<'static, str>, LogValue>;
pub type LogField = (Cow<'static, str>, LogValue);
pub type LogFieldRef<'a> = (&'a Cow<'static, str>, &'a LogValue);

/// A set of fields that can be attached to a logging scope.
///
/// [`LogFields`] represents a set of key-value pairs that can be
/// added to log messages when the log context scope is active.
///
/// # Ordering
///
/// The order in which fields appear is **not guaranteed**. Do not rely on any specific
/// ordering of keys.
#[derive(Debug, Clone, Default)]
pub struct LogFields(pub(crate) HashMap<Cow<'static, str>, LogValue>);

impl LogFields {
    /// Creates a new, empty set of fields.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a key-value pair into this collection, returning the collection for chained calls.
    ///
    /// This method takes ownership of `self`, so it can be used as part of a
    /// builder-style chain:
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogFields;
    ///
    /// let fields = LogFields::new()
    ///       .with("user_id", "user-123")
    ///       .with("request_id", 42);
    /// ```
    #[must_use]
    pub fn with(mut self, key: impl Into<Cow<'static, str>>, value: impl Into<LogValue>) -> Self {
        self.insert(key, value);
        self
    }

    /// Inserts a key-value pair into this collection.
    ///
    /// Unlike [`Self::with`], this method borrows `self` and
    /// returns a mutable reference, allowing it to be used when chaining with other methods
    /// that require borrowing.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogFields;
    ///
    /// let mut fields = LogFields::new();
    /// fields
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

    /// Merges this collection with the fields from another one.
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
    /// use context_logger::LogFields;
    ///
    /// # let other_fields = LogFields::new();
    /// let mut fields = LogFields::new();
    /// fields
    ///     .insert("user_id", "Alice")
    ///     .merge_with(other_fields)
    ///     .insert("request_id", 42);
    /// ```
    pub fn merge_with(&mut self, other: impl IntoIterator<Item = LogField>) -> &mut Self {
        self.0.extend(other);
        self
    }

    /// Returns an iterator over the fields in this collection.
    #[must_use]
    pub fn iter(&self) -> LogFieldsIter<'_> {
        self.0.iter()
    }

    /// Returns `true` if this collection contains no fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a LogFields {
    type Item = LogFieldRef<'a>;
    type IntoIter = LogFieldsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for LogFields {
    type Item = LogField;
    type IntoIter = LogFieldsIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Extend<LogField> for LogFields {
    fn extend<I: IntoIterator<Item = LogField>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl FromIterator<LogField> for LogFields {
    fn from_iter<T: IntoIterator<Item = LogField>>(iter: T) -> Self {
        Self(HashMap::from_iter(iter))
    }
}

#[cfg(test)]
impl LogFields {
    /// Returns a reference to the value associated with the given key, if it exists.
    pub(crate) fn find(&self, key: impl AsRef<str>) -> Option<&LogValue> {
        self.0.get(&Cow::Owned(key.as_ref().to_owned()))
    }
}

#[cfg(test)]
impl std::ops::Index<&str> for LogFields {
    type Output = LogValue;

    fn index(&self, index: &str) -> &Self::Output {
        self.0
            .get(&Cow::Owned(index.to_owned()))
            .expect("No value found for the given key")
    }
}
