//! Context builder for structured logging.

use std::borrow::Cow;

use crate::{LogValue, records::LogRecords};

/// A set of records that can be attached to a logging scope.
///
/// Records are split into two categories:
///
/// - **local** - records belonging only to the current scope.
///   They do not propagate to child scopes.
/// - **inherited** - records that automatically flow into all child scopes created within the current scope.
#[derive(Debug, Default, Clone)]
pub struct LogContext {
    /// Records belonging only to the current scope.
    pub local: LogRecords,
    /// Records that automatically flow into all child scopes created within the current scope.
    pub inherited: LogRecords,
}

impl LogContext {
    /// Creates a new, empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a key-value record to the local records of this context.
    ///
    /// See [`LogRecords`] for more details about log records.
    #[must_use]
    pub fn local_record(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<LogValue>,
    ) -> Self {
        self.local = self.local.field(key, value);
        self
    }

    /// Adds a key-value record to the inherited records of this context.
    ///
    /// See [`LogRecords`] for more details about log records.
    #[must_use]
    pub fn inherited_record(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<LogValue>,
    ) -> Self {
        self.inherited = self.inherited.field(key, value);
        self
    }

    /// Returns `true` if the both local and inherited records are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.local.0.is_empty()
    }
}
