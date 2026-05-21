//! A current logging context guard.

use std::{borrow::Cow, marker::PhantomData};

use crate::{
    LogContext, LogValue,
    stack::{SCOPE_STACK, ScopeStack},
};

/// A guard representing a current logging context in the context stack.
///
/// When the guard is dropped, the context is automatically removed from the stack.
/// This is returned by the [`LogContext::enter`] method.
///
/// # Examples
///
/// ```
/// use context_logger::{LogContext, LogScope};
///
/// // Create a context with some data
/// let context = LogContext::new().with_record("user_id", 123);
///
/// // Enter the context (pushes to stack)
/// let guard = LogScope::enter(context);
///
/// // Log operations here will have access to the context
/// // ...
///
/// // When `guard` goes out of scope, the context is automatically removed
/// ```
#[non_exhaustive]
#[derive(Debug)]
pub struct LogScope<'a> {
    // Make this guard unsendable.
    _marker: PhantomData<&'a *mut ()>,
}

impl LogScope<'_> {
    /// Activates this scope, returning a guard that will exit the scope when dropped.
    ///
    /// # In Asynchronous Code
    ///
    /// *Warning:* in asynchronous code [`Self::enter`] should be used very carefully or avoided entirely.
    /// Holding the drop guard across `.await` points will result in incorrect logs:
    ///
    /// ```rust
    /// use context_logger::{LogContext, LogScope};
    ///
    /// async fn my_async_fn() {
    ///     let ctx = LogContext::new()
    ///         .with_record("request_id", "req-123")
    ///         .with_record("user_id", 42);
    ///     // WARNING: This context will remain active until this
    ///     // guard is dropped...
    ///     let _guard = LogScope::enter(ctx);
    ///     // But this code causing the runtime to switch to another task,
    ///     // while remaining in this context.
    ///     tokio::task::yield_now().await;
    /// }
    /// ```
    ///
    /// Please use the [`crate::FutureExt::in_log_context`] instead.
    #[must_use]
    pub fn enter(context: LogContext) -> Self {
        SCOPE_STACK.with(|stack| stack.push(context.frame));
        Self {
            _marker: PhantomData,
        }
    }

    /// Adds a record to the currently active scope.
    ///
    /// This is useful for adding records dynamically without having
    /// direct access to the current scope.
    ///
    /// # Note
    ///
    /// If there is no active context, this operation will have no effect.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{LogContext, LogScope};
    /// use log::info;
    ///
    /// fn process_request() {
    ///     // Add a record to the current scope dynamically
    ///     LogScope::add_record("processing_time_ms", 42);
    ///     info!("Request processed");
    /// }
    ///
    /// let _guard = LogScope::enter(LogContext::new()
    ///     .with_record("request_id", "req-123"));
    ///
    /// process_request(); // Will log with both request_id and processing_time_ms
    /// ```
    pub fn add_record(key: impl Into<Cow<'static, str>>, value: impl Into<LogValue>) {
        SCOPE_STACK.with(|stack| {
            if let Some(mut top) = stack.top_mut() {
                let record = (key.into(), value.into());
                top.push(record);
            }
        });
    }

    pub(crate) fn exit(self) -> LogContext {
        // We need to prevent the destructor from being called
        // because we're manually managing the context stack here.
        std::mem::forget(self);

        let frame = SCOPE_STACK.with(ScopeStack::pop).expect(
            "bug in LogContextGuard::exit: expected a scope frame to exist when popping on exit",
        );
        LogContext { frame }
    }
}

#[derive(Debug)]
pub struct LogScopeGuard<'a> {
    // Make this guard unsendable.
    _marker: PhantomData<&'a *mut ()>,
}

impl Drop for LogScope<'_> {
    fn drop(&mut self) {
        SCOPE_STACK.with(ScopeStack::pop);
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::stack::SCOPE_STACK;

    #[test]
    fn test_log_context_guard_enter() {
        let context = LogContext::new().with_record("simple", 42);
        // Make sure the context stack is empty before entering the context.
        assert_eq!(SCOPE_STACK.with(ScopeStack::is_empty), true);

        let guard = LogScope::enter(context);
        // Check that the record was added to the top context.
        assert_eq!(
            SCOPE_STACK.with(|stack| stack.top().unwrap().records().len()),
            1
        );

        // Check that the context stack is empty after dropping the guard.
        drop(guard);
        assert_eq!(SCOPE_STACK.with(ScopeStack::len), 0);
    }

    #[test]
    fn test_log_context_nested_guards() {
        let outer_context = LogContext::new().with_record("simple_record", "outer_value");
        assert_eq!(SCOPE_STACK.with(ScopeStack::len), 0);

        let outer_guard = LogScope::enter(outer_context);
        assert_eq!(
            SCOPE_STACK.with(|stack| stack.top().unwrap().records().len()),
            1
        );

        SCOPE_STACK.with(|stack| {
            let frame = stack.top().unwrap();
            assert_eq!(
                frame.find("simple_record").unwrap().value().to_string(),
                "outer_value"
            );
        });

        let inner_context = LogContext::new().with_record("simple_record", "inner_value");
        {
            let inner_guard = LogScope::enter(inner_context);
            // Test log context after inner guard is entered.
            assert_eq!(SCOPE_STACK.with(ScopeStack::len), 2);
            SCOPE_STACK.with(|stack| {
                let frame = stack.top().unwrap();
                assert_eq!(
                    frame.find("simple_record").unwrap().value().to_string(),
                    "inner_value"
                );
            });

            drop(inner_guard);
        }
        // Test log context after inner guard is dropped.
        assert_eq!(
            SCOPE_STACK.with(|stack| stack.top().unwrap().records().len()),
            1
        );
        SCOPE_STACK.with(|stack| {
            let frame = stack.top().unwrap();
            assert_eq!(
                frame.find("simple_record").unwrap().value().to_string(),
                "outer_value"
            );
        });

        drop(outer_guard);
        assert_eq!(SCOPE_STACK.with(ScopeStack::is_empty), true);
    }

    #[test]
    fn test_log_context_multithread() {
        let local_context = LogContext::new().with_record("simple_record", "main");
        let local_guard = LogScope::enter(local_context);

        let first_thread_handle = std::thread::spawn(|| {
            let inner_context = LogContext::new().with_record("simple_record", "first_thread");
            let inner_guard = LogScope::enter(inner_context);

            // Test log context after inner guard is entered.
            assert_eq!(SCOPE_STACK.with(ScopeStack::len), 1);
            SCOPE_STACK.with(|stack| {
                let frame = stack.top().unwrap();
                assert_eq!(
                    frame.find("simple_record").unwrap().value().to_string(),
                    "first_thread"
                );
            });

            drop(inner_guard);
        });
        let second_thread_handle = std::thread::spawn(|| {
            let inner_context = LogContext::new().with_record("simple_record", "second_thread");
            let inner_guard = LogScope::enter(inner_context);

            // Test log context after inner guard is entered.
            assert_eq!(SCOPE_STACK.with(ScopeStack::len), 1);
            SCOPE_STACK.with(|stack| {
                let frame = stack.top().unwrap();
                assert_eq!(
                    frame.find("simple_record").unwrap().value().to_string(),
                    "second_thread"
                );
            });

            drop(inner_guard);
        });

        first_thread_handle.join().unwrap();
        second_thread_handle.join().unwrap();

        SCOPE_STACK.with(|stack| {
            let frame = stack.top().unwrap();
            assert_eq!(
                frame.find("simple_record").unwrap().value().to_string(),
                "main"
            );
        });
        drop(local_guard);
    }
}
