//! Context builder for structured logging.

use crate::{
    ContextValue, StaticCowStr,
    guard::LogContextGuard,
    stack::{CONTEXT_STACK, ContextFrame},
};

/// A contextual properties that can be attached to log records.
///
/// [`LogContext`] represents a set of key-value pairs that will be
/// automatically added to log messages when the context is active.
///
/// Records come in two flavours:
///
/// - **Local** records (added via [`record`](Self::record)) are visible only
///   while the context they belong to is the *active* context.  They do **not**
///   propagate into nested contexts.
/// - **Inherited** records (added via [`inherited_record`](Self::inherited_record))
///   are visible in the current context *and* in every nested context that is
///   activated while this context is on the stack.
#[derive(Debug, Clone)]
pub struct LogContext(pub(crate) ContextFrame);

impl LogContext {
    /// Creates a new, empty context.
    #[must_use]
    pub const fn new() -> Self {
        Self(ContextFrame::new())
    }

    /// Adds a local property to this context.
    ///
    /// Local records are only visible while this context is the **current**
    /// (innermost) active context.  They are not visible inside any nested
    /// context.
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
        self.0.local.push((key.into(), value.into()));
        self
    }

    /// Adds an inherited property to this context.
    ///
    /// Inherited records are visible both in this context **and** in every
    /// nested context that is activated while this context is on the stack.
    /// Use this when you want a property (e.g. `request_id`, `trace_id`) to
    /// be automatically carried into all nested scopes.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::LogContext;
    /// use log::info;
    ///
    /// let _guard = LogContext::new()
    ///     .inherited_record("request_id", "req-123") // visible in nested contexts too
    ///     .record("handler", "process_request")      // local only
    ///     .enter();
    ///
    /// let _inner = LogContext::new()
    ///     .record("step", "validate")
    ///     .enter();
    ///
    /// info!("validating"); // includes request_id (inherited) and step (local inner)
    ///                      // but NOT handler (local outer)
    /// ```
    #[must_use]
    pub fn inherited_record(
        mut self,
        key: impl Into<StaticCowStr>,
        value: impl Into<ContextValue>,
    ) -> Self {
        self.0.inherited.push((key.into(), value.into()));
        self
    }

    /// Adds a local property to the current active context.
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

    /// Adds an inherited property to the current active context.
    ///
    /// Unlike [`add_record`](Self::add_record), the added property will be visible
    /// in every nested context that is activated after this call.
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
    /// fn set_trace_id(trace_id: &'static str) {
    ///     LogContext::add_inherited_record("trace_id", trace_id);
    /// }
    ///
    /// let _outer = LogContext::new().enter();
    /// set_trace_id("trace-abc");
    ///
    /// let _inner = LogContext::new().record("step", "validate").enter();
    /// info!("validating"); // includes trace_id (inherited) and step (local)
    /// ```
    pub fn add_inherited_record(key: impl Into<StaticCowStr>, value: impl Into<ContextValue>) {
        let property = (key.into(), value.into());

        CONTEXT_STACK.with(|stack| {
            if let Some(mut top) = stack.top_inherited_mut() {
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
