//! Provides RAII guards for safely managing the logging context stack.
//!
//! This module contains the [`LogContextGuard`] type, which is responsible for
//! managing the lifecycle of a logging context. When a guard is created, it adds
//! the context to the thread-local stack, and when the guard is dropped, it
//! automatically removes the context.

use std::marker::PhantomData;

use crate::{
    LogContext,
    stack::{CONTEXT_STACK, ContextStack},
};

/// A RAII guard that manages the lifecycle of a logging context in the context stack.
///
/// `LogContextGuard` provides automatic cleanup of logging contexts using Rust's
/// ownership system. When a guard is dropped (goes out of scope), the associated
/// context is automatically removed from the stack.
///
/// This guard is created by the [`LogContext::enter`] method and should not be
/// constructed directly.
///
/// # Examples
///
/// ```
/// use context_logger::LogContext;
/// use log::info;
///
/// // Create a context with some data
/// let context = LogContext::new().record("user_id", 123);
///
/// // Enter the context (pushes to stack)
/// let guard = context.enter();
///
/// // Log operations here will have access to the context
/// info!("Processing data for user");
///
/// // When `guard` goes out of scope, the context is automatically removed
/// ```
///
/// # Thread Safety
///
/// The guard is intentionally not `Send` to ensure that contexts are only
/// used on the thread where they were created. For propagating context across
/// thread boundaries, use thread-specific contexts.
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
        assert_eq!(
            CONTEXT_STACK.with(|stack| stack.top().unwrap().properties.len()),
            1
        );

        // Check that the context stack is empty after dropping the guard.
        drop(guard);
        assert_eq!(CONTEXT_STACK.with(ContextStack::len), 0);
    }

    #[test]
    fn test_log_context_nested_guards() {
        let outer_context = LogContext::new().record("simple_record", "outer_value");
        assert_eq!(CONTEXT_STACK.with(ContextStack::len), 0);

        let outer_guard = outer_context.enter();
        assert_eq!(
            CONTEXT_STACK.with(|stack| stack.top().unwrap().properties.len()),
            1
        );

        CONTEXT_STACK.with(|stack| {
            let property = &stack.top().unwrap().properties[0];
            assert_eq!(property.0, "simple_record");
            assert_eq!(property.1.to_string(), "outer_value");
        });

        let inner_context = LogContext::new().record("simple_record", "inner_value");
        {
            let inner_guard = inner_context.enter();
            // Test log context after inner guard is entered.
            assert_eq!(CONTEXT_STACK.with(ContextStack::len), 2);
            CONTEXT_STACK.with(|stack| {
                let property = &stack.top().unwrap().properties[0];
                assert_eq!(property.0, "simple_record");
                assert_eq!(property.1.to_string(), "inner_value");
            });

            drop(inner_guard);
        }
        // Test log context after inner guard is dropped.
        assert_eq!(
            CONTEXT_STACK.with(|stack| stack.top().unwrap().properties.len()),
            1
        );
        CONTEXT_STACK.with(|stack| {
            let property = &stack.top().unwrap().properties[0];
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
                let property = &stack.top().unwrap().properties[0];
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
                let property = &stack.top().unwrap().properties[0];
                assert_eq!(property.0, "simple_record");
                assert_eq!(property.1.to_string(), "second_thread");
            });

            drop(inner_guard);
        });

        first_thread_handle.join().unwrap();
        second_thread_handle.join().unwrap();

        CONTEXT_STACK.with(|stack| {
            let property = &stack.top().unwrap().properties[0];
            assert_eq!(property.0, "simple_record");
            assert_eq!(property.1.to_string(), "main");
        });
        drop(local_guard);
    }
}
