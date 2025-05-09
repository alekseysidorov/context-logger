//! Context management for structured logging.
//!
//! This module provides functionality for creating, managing, and attaching
//! structured context data to log records. The primary type is [`LogContext`],
//! which serves as a container for key-value properties that are automatically
//! added to log records when in scope.

use crate::{
    ContextValue, StaticCowStr,
    guard::LogContextGuard,
    stack::{CONTEXT_STACK, ContextProperties},
};

/// A container for contextual properties that can be attached to log records.
///
/// `LogContext` represents a set of key-value pairs that will be automatically
/// added to log messages when the context is active. Contexts can be nested,
/// creating a hierarchy of properties that are available to log records.
///
/// # Examples
///
/// ```
/// use context_logger::LogContext;
/// use log::info;
///
/// // Create a context with properties
/// let ctx = LogContext::new()
///     .record("request_id", "req-123")
///     .record("user_id", 42);
///
/// // Enter the context (making it active for logging)
/// let _guard = ctx.enter();
///
/// // Log messages will now include context properties
/// info!("Processing request");
/// ```
#[derive(Debug)]
pub struct LogContext(pub(crate) ContextProperties);

impl LogContext {
    /// Creates a new, empty context.
    ///
    /// The context starts with no properties. Use [`record`](Self::record) to add
    /// properties to the context.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogContext;
    ///
    /// let context = LogContext::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self(ContextProperties::new())
    }

    /// Adds a property to this context.
    ///
    /// This method takes ownership of the context and returns it, allowing for
    /// method chaining when building a context with multiple properties.
    ///
    /// # Parameters
    ///
    /// * `key` - The name of the property to add
    /// * `value` - The value to associate with the key
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
    pub fn record(mut self, key: impl Into<StaticCowStr>, value: impl Into<ContextValue>) -> Self {
        let property = (key.into(), value.into());
        self.0.properties.push(property);
        self
    }

    /// Adds a property to the current active context.
    ///
    /// This static method adds a property to the top context on the thread-local
    /// context stack, if one exists. This is useful for adding context information
    /// dynamically from within a function without having direct access to the context.
    ///
    /// # Parameters
    ///
    /// * `key` - The name of the property to add
    /// * `value` - The value to associate with the key
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{LogContext, ContextValue};
    /// use log::info;
    ///
    /// fn process_request() {
    ///     // Add context information dynamically
    ///     LogContext::add_record("processing_time_ms", 42);
    ///     info!("Request processed");
    /// }
    ///
    /// let _guard = LogContext::new()
    ///     .record("request_id", "req-123")
    ///     .enter();
    ///
    /// process_request(); // Will log with both request_id and processing_time_ms
    /// ```
    ///
    /// # Note
    ///
    /// If there is no active context, this operation will have no effect.
    pub fn add_record(key: impl Into<StaticCowStr>, value: impl Into<ContextValue>) {
        let property = (key.into(), value.into());

        CONTEXT_STACK.with(|stack| {
            if let Some(mut top) = stack.top_mut() {
                top.properties.push(property);
            }
        });
    }

    /// Activates this context for the current thread.
    ///
    /// This method pushes the context onto the thread-local context stack and returns
    /// a guard that will automatically remove the context when dropped. While the
    /// context is active, all log messages emitted from the thread will include
    /// the context properties.
    ///
    /// # Returns
    ///
    /// A guard that removes the context when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogContext;
    /// use log::info;
    ///
    /// {
    ///     let _guard = LogContext::new()
    ///         .record("user_id", "user-123")
    ///         .enter();
    ///
    ///     info!("User logged in"); // Will include user_id
    /// } // Context is removed when guard is dropped
    ///
    /// info!("Operation complete"); // Won't include user_id
    /// ```
    #[must_use]
    pub fn enter<'a>(self) -> LogContextGuard<'a> {
        LogContextGuard::enter(self)
    }
}

impl Default for LogContext {
    fn default() -> Self {
        Self::new()
    }
}
