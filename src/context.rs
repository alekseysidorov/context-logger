//! Context builder for structured logging.

use std::borrow::Cow;

use crate::{LogValue, fields::LogFields};

/// A set of fields that can be attached to a logging scope.
///
/// Fields are split into two categories:
///
/// - **local** - fields belonging only to the current scope. They do not propagate to child scopes.
/// - **inherited** - fields that automatically flow into all child scopes created within the
///   current scope.
///
/// Nested scopes resolution rules:
///
/// - child `inherited` overwrites parent `inherited` by key
/// - inherited are emitted before local, so "last write wins" consumers see local shadowing
#[derive(Debug, Default, Clone)]
pub struct LogContext {
    /// Fields belonging only to the current scope.
    pub local: LogFields,
    /// Fields that automatically flow into all child scopes created within the
    /// current scope.
    pub inherited: LogFields,
}

impl LogContext {
    /// Creates a new, empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a key-value field to the local fields of this context.
    ///
    /// See [`LogFields`] for more details about log fields.
    #[must_use]
    pub fn with_local_field(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<LogValue>,
    ) -> Self {
        self.local = self.local.with(key, value);
        self
    }

    /// Adds a key-value pair to the inherited fields of this context.
    ///
    /// See [`LogFields`] for more details about log fields.
    #[must_use]
    pub fn with_inherited_field(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<LogValue>,
    ) -> Self {
        self.inherited = self.inherited.with(key, value);
        self
    }

    /// Returns `true` if both local and inherited fields are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.local.is_empty() && self.inherited.is_empty()
    }
}
