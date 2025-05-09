//! A current logging context guard.

use std::marker::PhantomData;

use crate::{
    stack::{ContextStack, CONTEXT_STACK},
    LogContext,
};

/// A guard representing a current logging context in the context stack.
///
/// When the guard is dropped, the context is automatically removed from the stack.
/// This is returned by the [`LogContext::enter`] method.
///
/// # Examples
///
/// ```
/// use context_logger::LogContext;
///
/// // Create a context with some data
/// let context = LogContext::new().record("user_id", 123);
///
/// // Enter the context (pushes to stack)
/// let guard = context.enter();
///
/// // Log operations here will have access to the context
/// // ...
///
/// // When `guard` goes out of scope, the context is automatically removed
/// ```
#[non_exhaustive]
#[derive(Debug)]
pub struct LogContextGuard<'a> {
    // Make this guard unsendable.
    _marker: PhantomData<&'a *mut ()>,
}

impl LogContextGuard<'_> {
    pub(crate) fn enter(context: LogContext) -> Self {
        CONTEXT_STACK.with(|stack| stack.push(context.0));
        Self {
            _marker: PhantomData,
        }
    }

    pub(crate) fn exit(self) -> LogContext {
        // We need to prevent the destructor from being called
        // because we're manually managing the context stack here.
        std::mem::forget(self);

        let properties = CONTEXT_STACK
            .with(ContextStack::pop)
            .expect("There is a bug in log context guard, context should be exists");
        LogContext(properties)
    }
}

impl Drop for LogContextGuard<'_> {
    fn drop(&mut self) {
        CONTEXT_STACK.with(ContextStack::pop);
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::stack::CONTEXT_STACK;

    #[test]
    fn test_log_context_guard_enter() {
        let context = LogContext::new().record("simple", 42);
        // Make sure the context stack is empty before entering the context.
        assert_eq!(CONTEXT_STACK.with(ContextStack::is_empty), true);

        let guard = context.enter();
        // Check that the record was added to the top context.
        assert_eq!(CONTEXT_STACK.with(|stack| stack.top().unwrap().len()), 1);

        // Check that the context stack is empty after dropping the guard.
        drop(guard);
        assert_eq!(CONTEXT_STACK.with(ContextStack::len), 0);
    }

    #[test]
    fn test_log_context_nested_guards() {
        let outer_context = LogContext::new().record("simple_record", "outer_value");
        assert_eq!(CONTEXT_STACK.with(ContextStack::len), 0);

        let outer_guard = outer_context.enter();
        assert_eq!(CONTEXT_STACK.with(|stack| stack.top().unwrap().len()), 1);

        CONTEXT_STACK.with(|stack| {
            let property = &stack.top().unwrap()[0];
            assert_eq!(property.0, "simple_record");
            assert_eq!(property.1.to_string(), "outer_value");
        });

        let inner_context = LogContext::new().record("simple_record", "inner_value");
        {
            let inner_guard = inner_context.enter();
            // Test log context after inner guard is entered.
            assert_eq!(CONTEXT_STACK.with(ContextStack::len), 2);
            CONTEXT_STACK.with(|stack| {
                let property = &stack.top().unwrap()[0];
                assert_eq!(property.0, "simple_record");
                assert_eq!(property.1.to_string(), "inner_value");
            });

            drop(inner_guard);
        }
        // Test log context after inner guard is dropped.
        assert_eq!(CONTEXT_STACK.with(|stack| stack.top().unwrap().len()), 1);
        CONTEXT_STACK.with(|stack| {
            let property = &stack.top().unwrap()[0];
            assert_eq!(property.0, "simple_record");
            assert_eq!(property.1.to_string(), "outer_value");
        });

        drop(outer_guard);
        assert_eq!(CONTEXT_STACK.with(ContextStack::is_empty), true);
    }

    #[test]
    fn test_log_context_multithread() {
        let local_context = LogContext::new().record("simple_record", "main");
        let local_guard = local_context.enter();

        let first_thread_handle = std::thread::spawn(|| {
            let inner_context = LogContext::new().record("simple_record", "first_thread");
            let inner_guard = inner_context.enter();

            // Test log context after inner guard is entered.
            assert_eq!(CONTEXT_STACK.with(ContextStack::len), 1);
            CONTEXT_STACK.with(|stack| {
                let property = &stack.top().unwrap()[0];
                assert_eq!(property.0, "simple_record");
                assert_eq!(property.1.to_string(), "first_thread");
            });

            drop(inner_guard);
        });
        let second_thread_handle = std::thread::spawn(|| {
            let inner_context = LogContext::new().record("simple_record", "second_thread");
            let inner_guard = inner_context.enter();
            // Test log context after inner guard is entered.
            assert_eq!(CONTEXT_STACK.with(ContextStack::len), 1);
            CONTEXT_STACK.with(|stack| {
                let property = &stack.top().unwrap()[0];
                assert_eq!(property.0, "simple_record");
                assert_eq!(property.1.to_string(), "second_thread");
            });

            drop(inner_guard);
        });

        first_thread_handle.join().unwrap();
        second_thread_handle.join().unwrap();

        CONTEXT_STACK.with(|stack| {
            let property = &stack.top().unwrap()[0];
            assert_eq!(property.0, "simple_record");
            assert_eq!(property.1.to_string(), "main");
        });
        drop(local_guard);
    }
}
