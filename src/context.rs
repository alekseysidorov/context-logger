//! Context builder for structured logging.

use std::borrow::Cow;

use crate::{
    LogValue,
    stack::{SCOPE_STACK, ScopeFrame},
};

/// A set of records that can be attached to a logging scope.
///
/// [`LogContext`] represents a set of key-value pairs that will be
/// automatically added to log messages when the context is active.
#[derive(Debug, Clone)]
pub struct LogContext {
    pub(crate) frame: ScopeFrame,
}

impl LogContext {
    /// Creates a new, empty context.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            frame: ScopeFrame::new(),
        }
    }

    /// Adds a record to this scope.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogContext;
    ///
    /// let context = LogContext::new()
    ///     .record("user_id", "user-123")
    ///     .record("request_id", 42)
    ///     .record("is_admin", true);
    /// ```
    #[must_use]
    pub fn record(mut self, key: impl Into<Cow<'static, str>>, value: impl Into<LogValue>) -> Self {
        let record = (key, value);
        self.frame.push(record);
        self
    }
}

impl Default for LogContext {
    fn default() -> Self {
        Self::new()
    }
}
