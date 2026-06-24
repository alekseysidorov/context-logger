//! A current logging context guard.

use std::{borrow::Cow, marker::PhantomData};

use self::stack::{SCOPE_STACK, ScopeStack};
use crate::{LogContext, LogValue};

pub mod stack;

/// A guard that represents an active logging context on the current thread's scope stack.
///
/// When the guard is dropped, the context is automatically removed from the stack.
/// Created by [`LogScope::enter`].
///
/// # Examples
///
/// ```
/// use context_logger::{LogContext, LogScope};
///
/// // Create a context with some data
/// let context = LogContext::new().with_local_record("user_id", 123);
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
pub struct LogScope {
    // Make this guard non-Send: LogScope manages thread-local state
    // and must not be transferred to another thread.
    _marker: PhantomData<*mut ()>,
}

impl LogScope {
    /// Pushes the given context onto the current thread's scope stack and returns a guard.
    ///
    /// The context remains active until the returned guard is dropped, at which point
    /// it is automatically removed from the stack.
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
    ///         .with_local_record("request_id", "req-123")
    ///         .with_local_record("user_id", 42);
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
        SCOPE_STACK.with(|stack| stack.push(context));
        Self {
            _marker: PhantomData,
        }
    }

    /// Enters the given context, runs a closure, and exits the scope automatically.
    ///
    /// This is a convenience method for short synchronous sections where context
    /// should be active only during closure execution.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{LogContext, LogScope};
    ///
    /// let context = LogContext::new().with_local_record("request_id", "req-123");
    /// let result = LogScope::in_scope(
    ///     context,
    ///     || 40 + 2,
    /// );
    ///
    /// assert_eq!(result, 42);
    /// ```
    pub fn in_scope<R>(context: LogContext, f: impl FnOnce() -> R) -> R {
        let _guard = Self::enter(context);
        f()
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
    /// # Ordering
    ///
    /// The order in which records appear in log output is **not guaranteed**.
    /// Do not rely on any specific ordering of keys.
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
    ///     .with_local_record("request_id", "req-123"));
    ///
    /// process_request(); // Will log with both request_id and processing_time_ms
    /// ```
    pub fn add_record(key: impl Into<Cow<'static, str>>, value: impl Into<LogValue>) {
        SCOPE_STACK.with(|stack| {
            if let Some(mut top) = stack.top_mut() {
                top.0.local.insert(key, value);
            }
        });
    }

    /// Extracts the currently active logging context.
    ///
    /// This is useful for propagating context when spawning new threads or async tasks,
    /// allowing child tasks to inherit logging information from the current scope.
    ///
    /// # Example
    ///
    /// ```no_run
    #[doc = include_str!("../../examples/current_context.rs")]
    /// ```
    ///
    /// # Notes
    ///
    /// - Returns an empty context if there is no active scope.
    /// - The returned [`LogContext`] is a clone of the active context, so it's safe to move into spawned tasks.
    #[must_use]
    pub fn current_context() -> LogContext {
        SCOPE_STACK
            .with(|stack| stack.top().map(|frame| frame.clone().into()))
            .unwrap_or_default()
    }

    pub(crate) fn exit(self) -> LogContext {
        // We need to prevent the destructor from being called
        // because we're manually managing the context stack here.
        std::mem::forget(self);

        let frame = SCOPE_STACK
            .with(ScopeStack::pop)
            .expect("bug in LogScope::exit: expected a scope frame to exist when popping on exit");
        frame.into()
    }
}

impl Drop for LogScope {
    fn drop(&mut self) {
        SCOPE_STACK.with(ScopeStack::pop);
    }
}

/// Extension trait for [`LogContext`] to run code within a temporary logging scope.
///
/// This trait provides ergonomic, method-style access to [`LogScope::in_scope`].
pub trait LogContextExt: Sized + crate::private::Sealed {
    /// Enters this context, runs a closure, and exits the scope automatically.
    ///
    /// This is equivalent to calling [`LogScope::in_scope`] with `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{LogContext, LogContextExt as _};
    ///
    /// let result = LogContext::new()
    ///     .with_local_record("request_id", "req-123")
    ///     .in_scope(|| 40 + 2);
    ///
    /// assert_eq!(result, 42);
    /// ```
    fn in_scope<R>(self, f: impl FnOnce() -> R) -> R;
}

impl LogContextExt for LogContext {
    fn in_scope<R>(self, f: impl FnOnce() -> R) -> R {
        LogScope::in_scope(self, f)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use static_assertions::assert_not_impl_any;

    use super::*;

    // LogScope manages thread-local state and must never be Send.
    assert_not_impl_any!(LogScope: Send);

    #[test]
    fn test_log_context_guard_enter() {
        let context = LogContext::new().with_local_record("simple", 42);
        // Make sure the context stack is empty before entering the context.
        assert_eq!(SCOPE_STACK.with(ScopeStack::is_empty), true);

        let guard = LogScope::enter(context);
        // Check that the record was added to the top context.
        assert_eq!(
            SCOPE_STACK.with(|stack| stack.top().unwrap().records().count()),
            1
        );

        // Check that the context stack is empty after dropping the guard.
        drop(guard);
        assert_eq!(SCOPE_STACK.with(ScopeStack::len), 0);
    }

    #[test]
    fn test_log_context_nested_guards() {
        let outer_context = LogContext::new().with_local_record("simple_record", "outer_value");
        assert_eq!(SCOPE_STACK.with(ScopeStack::len), 0);

        let outer_guard = LogScope::enter(outer_context);
        assert_eq!(
            SCOPE_STACK.with(|stack| stack.top().unwrap().records().count()),
            1
        );

        SCOPE_STACK.with(|stack| {
            let context = &stack.top().unwrap().0;
            assert_eq!(
                context.local.0.get("simple_record").unwrap().to_string(),
                "outer_value"
            );
        });

        let inner_context = LogContext::new().with_local_record("simple_record", "inner_value");
        {
            let inner_guard = LogScope::enter(inner_context);
            // Test log context after inner guard is entered.
            assert_eq!(SCOPE_STACK.with(ScopeStack::len), 2);
            SCOPE_STACK.with(|stack| {
                let frame = stack.top().unwrap();
                assert_eq!(
                    frame.0.local.find("simple_record").unwrap().to_string(),
                    "inner_value"
                );
            });

            drop(inner_guard);
        }
        // Test log context after inner guard is dropped.
        assert_eq!(
            SCOPE_STACK.with(|stack| stack.top().unwrap().records().count()),
            1
        );
        SCOPE_STACK.with(|stack| {
            let frame = stack.top().unwrap();
            assert_eq!(
                frame.0.local.find("simple_record").unwrap().to_string(),
                "outer_value"
            );
        });

        drop(outer_guard);
        assert_eq!(SCOPE_STACK.with(ScopeStack::is_empty), true);
    }

    #[test]
    fn test_log_context_multithread() {
        let local_context = LogContext::new().with_local_record("simple_record", "main");
        let local_guard = LogScope::enter(local_context);

        let first_thread_handle = std::thread::spawn(|| {
            let inner_context =
                LogContext::new().with_local_record("simple_record", "first_thread");
            let inner_guard = LogScope::enter(inner_context);

            // Test log context after inner guard is entered.
            assert_eq!(SCOPE_STACK.with(ScopeStack::len), 1);
            SCOPE_STACK.with(|stack| {
                let frame = stack.top().unwrap();
                assert_eq!(
                    frame.0.local.find("simple_record").unwrap().to_string(),
                    "first_thread"
                );
            });

            drop(inner_guard);
        });
        let second_thread_handle = std::thread::spawn(|| {
            let inner_context =
                LogContext::new().with_local_record("simple_record", "second_thread");
            let inner_guard = LogScope::enter(inner_context);

            // Test log context after inner guard is entered.
            assert_eq!(SCOPE_STACK.with(ScopeStack::len), 1);
            SCOPE_STACK.with(|stack| {
                let frame = stack.top().unwrap();
                assert_eq!(
                    frame.0.local.find("simple_record").unwrap().to_string(),
                    "second_thread"
                );
            });

            drop(inner_guard);
        });

        first_thread_handle.join().unwrap();
        second_thread_handle.join().unwrap();

        SCOPE_STACK.with(|stack| {
            let frame = stack.top().unwrap();
            assert_eq!(frame.0.local["simple_record"].to_string(), "main");
        });
        drop(local_guard);
    }

    #[test]
    fn test_current_context_empty_scope() {
        let context = LogScope::current_context();
        assert!(context.is_empty());
    }

    #[test]
    fn test_current_context_with_scope() {
        let context = LogContext::new().with_local_record("record", 42);
        {
            let _guard = LogScope::enter(context);

            let current_context = LogScope::current_context();
            assert_eq!(current_context.local["record"].to_string(), "42");
        }

        assert!(LogScope::current_context().is_empty());
    }

    #[test]
    fn test_in_scope_enters_context_and_returns_result() {
        assert!(SCOPE_STACK.with(ScopeStack::is_empty));

        let result = LogScope::in_scope(LogContext::new().with_local_record("record", 42), || {
            let current_context = LogScope::current_context();
            assert_eq!(current_context.local["record"].to_string(), "42");

            40 + 2
        });

        assert_eq!(result, 42);
        assert!(SCOPE_STACK.with(ScopeStack::is_empty));
    }

    #[test]
    fn test_log_context_ext_in_scope_enters_context_and_returns_result() {
        assert!(SCOPE_STACK.with(ScopeStack::is_empty));

        let result = LogContext::new()
            .with_local_record("record", 42)
            .in_scope(|| {
                let current_context = LogScope::current_context();
                assert_eq!(current_context.local["record"].to_string(), "42");

                40 + 2
            });

        assert_eq!(result, 42);
        assert!(SCOPE_STACK.with(ScopeStack::is_empty));
    }

    #[test]
    fn test_log_context_inherited_records() {
        LogContext::new()
            .with_local_record("name", "Ann")
            .with_inherited_record("tag", "42")
            .with_inherited_record("target", "root")
            .in_scope(|| {
                let ctx = LogScope::current_context();

                assert_eq!(ctx.local["name"].to_string(), "Ann");
                assert_eq!(ctx.inherited["tag"].to_string(), "42");
                assert_eq!(ctx.inherited["target"].to_string(), "root");

                LogContext::new()
                    .with_local_record("target", "nested")
                    .in_scope(|| {
                        let ctx = LogScope::current_context();

                        assert_eq!(ctx.local["target"].to_string(), "nested");
                        assert_eq!(ctx.inherited["tag"].to_string(), "42");
                        assert!(ctx.local.find("name").is_none());
                    });
            });
    }

    // Edge case: panic in child scope doesn't break parent stack
    #[test]
    fn test_panic_in_child_scope_does_not_break_parent() {
        // Push parent frame onto the stack
        let outer_context = LogContext::new()
            .with_inherited_record("outer", "val")
            .with_local_record("outer_local", "ol");
        {
            let _parent_guard = LogScope::enter(outer_context);
            // Verify parent is on the stack
            assert_eq!(SCOPE_STACK.with(ScopeStack::len), 1);

            // Panic in inner scope — the child guard's Drop must run
            let result = std::panic::catch_unwind(|| {
                LogContext::new().in_scope(|| panic!("inner panic"));
            });

            assert!(result.is_err());
        }

        // Stack must be clean: parent guard dropped + child guard's Drop ran
        assert_eq!(SCOPE_STACK.with(ScopeStack::len), 0);
    }

    // Edge case: two siblings from one parent each get their own inherited copy
    #[test]
    fn test_sibling_scopes_get_independent_inherited_copies() {
        let parent_ctx = LogContext::new()
            .with_inherited_record("parent_key", "pv")
            .with_local_record("parent_local", "pl");

        {
            let _g1 = LogScope::enter(parent_ctx);
            assert_eq!(SCOPE_STACK.with(ScopeStack::len), 1);

            // child1: inherits parent's `parent_key`, adds its own `sibling` and local
            let child1_result = LogContext::new()
                .with_inherited_record("sibling", "child1")
                .with_local_record("only_in_child1", "c1")
                .in_scope(|| {
                    let c = LogScope::current_context();
                    format!(
                        "{}|{}",
                        c.inherited["parent_key"], c.local["only_in_child1"]
                    )
                });
            assert_eq!(child1_result, "pv|c1");

            // after child1 scope ends, parent is still the only frame on the stack
            assert_eq!(SCOPE_STACK.with(ScopeStack::len), 1);

            // child2: also inherits parent's `parent_key`, but its own `sibling` wins
            let c2_result = LogContext::new()
                .with_inherited_record("sibling", "child2")
                .with_local_record("only_in_child2", "c2")
                .in_scope(|| {
                    let c = LogScope::current_context();
                    format!("{}|{}", c.inherited["parent_key"], c.inherited["sibling"])
                });
            assert_eq!(c2_result, "pv|child2");

            // parent state unchanged after child2 scope ends
            assert_eq!(SCOPE_STACK.with(ScopeStack::len), 1);
        }

        // After parent scope: stack is empty
        assert_eq!(SCOPE_STACK.with(ScopeStack::len), 0);
    }
}
