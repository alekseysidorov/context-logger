use crate::{
    ContextValue, StaticCowStr,
    guard::LogContextGuard,
    stack::{CONTEXT_STACK, ContextProperties},
};

/// A builder for creating and managing contextual properties for log entries.
///
/// `LogContext` allows you to attach key-value pairs to all log statements within
/// a given scope. The context can be entered for synchronous code using the `enter()`
/// method, or propagated through asynchronous tasks using the `FutureExt` trait.
///
/// # Examples
///
/// ```
/// use context_logger::LogContext;
/// use log::info;
///
/// // Create and enter a context
/// {
///     let _guard = LogContext::new()
///         .record("request_id", "abc-123")
///         .record("user_id", 42)
///         .enter();
///
///     // All logs within this scope will contain the context properties
///     info!("Processing request");
/// }
/// ```
#[derive(Debug)]
pub struct LogContext(pub(crate) ContextProperties);

impl LogContext {
    /// Creates a new, empty log context.
    ///
    /// This is typically the first step in building a context that will be
    /// populated with key-value pairs using the `record()` method.
    #[must_use]
    pub const fn new() -> Self {
        Self(ContextProperties::new())
    }

    /// Adds a key-value pair to this context.
    ///
    /// This method allows chaining multiple `record()` calls to build up a context
    /// with multiple properties. The value is converted to a [`ContextValue`], which
    /// supports various data types.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogContext;
    ///
    /// let context = LogContext::new()
    ///     .record("user_id", 123)
    ///     .record("request_path", "/api/data")
    ///     .record("is_admin", false);
    /// ```
    #[must_use]
    pub fn record(mut self, key: impl Into<StaticCowStr>, value: impl Into<ContextValue>) -> Self {
        let property = (key.into(), value.into());
        self.0.properties.push(property);
        self
    }

    /// Adds a key-value pair to the current active context.
    ///
    /// This is a static method that adds a property to the context at the top of
    /// the context stack. This is useful for dynamically adding information to
    /// the current context without having direct access to the context object.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{LogContext, ContextValue};
    /// use log::info;
    ///
    /// // Within a context...
    /// {
    ///     let _guard = LogContext::new().record("operation", "process_data").enter();
    ///     
    ///     // Some processing...
    ///     
    ///     // Add more information to the current context
    ///     LogContext::add_record("duration_ms", 157);
    ///     LogContext::add_record("status", "success");
    ///     
    ///     info!("Operation complete"); // Will include all context properties
    /// }
    /// ```
    ///
    /// If there is no active context, this method has no effect.
    pub fn add_record(key: impl Into<StaticCowStr>, value: impl Into<ContextValue>) {
        let property = (key.into(), value.into());

        CONTEXT_STACK.with(|stack| {
            if let Some(mut top) = stack.top_mut() {
                top.properties.push(property);
            }
        });
    }

    /// Enters this context, making it active for the current scope.
    ///
    /// This method returns a [`LogContextGuard`] that will automatically remove the
    /// context when dropped. All log statements within the scope where this guard
    /// exists will have access to the context properties.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogContext;
    /// use log::info;
    ///
    /// {
    ///     let _guard = LogContext::new()
    ///         .record("transaction_id", "tx-456")
    ///         .enter();
    ///         
    ///     // All logs within this scope will include the transaction_id
    ///     info!("Transaction started");
    ///     
    ///     // More code and logging...
    /// } // Context is automatically removed when guard is dropped
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
