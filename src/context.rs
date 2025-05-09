//! Context builder for structured logging.

use crate::{
    guard::LogContextGuard,
    stack::{ContextProperties, CONTEXT_STACK},
    ContextValue, StaticCowStr,
};

/// A contextual properties that can be attached to log records.
///
/// [`LogContext`] represents a set of key-value pairs that will be
/// automatically added to log messages when the context is active.
#[derive(Debug)]
pub struct LogContext(pub(crate) ContextProperties);

impl LogContext {
    /// Creates a new, empty context.
    #[must_use]
    pub const fn new() -> Self {
        Self(ContextProperties::new())
    }

    /// Adds property to this context.
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
        self.0.push(property);
        self
    }

    /// Adds property to the current active context.
    ///
    /// This is useful for adding context information dynamically without having
    /// direct access to the context.
    ///
    /// # Note
    ///
    /// If there is no active context, this operation will have no effect.
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
    pub fn add_record(key: impl Into<StaticCowStr>, value: impl Into<ContextValue>) {
        let property = (key.into(), value.into());

        CONTEXT_STACK.with(|stack| {
            if let Some(mut top) = stack.top_mut() {
                top.push(property);
            }
        });
    }

    /// Activating this context, returning a guard that will exit the context when dropped.
    ///
    /// # In Asynchronous Code
    ///
    /// *Warning:* in asynchronous code [`Self::enter`] should be used very carefully or avoided entirely.
    /// Holding the drop guard across `.await` points will result in incorrect logs:
    ///
    /// ```rust
    /// use context_logger::LogContext;
    ///
    /// async fn my_async_fn() {
    ///     let ctx = LogContext::new()
    ///         .record("request_id", "req-123")
    ///         .record("user_id", 42);
    ///     // WARNING: This context will remain active until this
    ///     // guard is dropped...
    ///     let _guard = ctx.enter();
    ///     // But this code causing the runtime to switch to another task,
    ///     // while remaining in this context.
    ///     tokio::task::yield_now().await;
    ///     }
    /// ```
    ///
    /// Please use the [`crate::FutureExt::in_log_context`] instead.
    ///
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
