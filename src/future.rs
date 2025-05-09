//! Asynchronous context propagation for structured logging.
//!
//! This module provides functionality for propagating log contexts through async
//! code, ensuring that log records emitted from async tasks include the
//! appropriate context information.
//!
//! The primary functionality is exposed through the [`FutureExt`] trait, which adds
//! the [`in_log_context`](FutureExt::in_log_context) method to any [`Future`].

use std::{future::Future, task::Poll};

use pin_project::pin_project;

use crate::{
    LogContext,
    stack::{CONTEXT_STACK, ContextProperties},
};

/// Extension trait for [`Future`]s that provides context propagation.
///
/// This trait extends the behavior of Rust's async futures by allowing a log context
/// to be attached to them. When the future is polled, the context is automatically
/// pushed onto the context stack, and when the poll ends, the context is popped off again.
///
/// This ensures that any log records emitted during the execution of the future will
/// include the appropriate context information, even across `.await` points.
///
/// # Examples
///
/// ```
/// use context_logger::{LogContext, FutureExt};
/// use log::info;
///
/// async fn process_user(user_id: &str) {
///     // Create a context with user_id
///     let context = LogContext::new().record("user_id", user_id);
///     
///     // Apply the context to the async block
///     async {
///         info!("Processing user"); // Has access to user_id
///         
///         // Context persists across await points
///         async_operation().await;
///         
///         info!("User processed"); // Still has access to user_id
///     }
///     .in_log_context(context)
///     .await;
/// }
///
/// async fn async_operation() {
///     // All logs emitted here will have access to the context
///     info!("Performing operation");
/// }
/// ```
pub trait FutureExt: Future + Sized {
    /// Attaches a log context to a future.
    ///
    /// When the future is polled, the context is automatically pushed onto the
    /// thread-local context stack. When polling completes, the context is popped
    /// off the stack. This ensures that any log records emitted during the execution
    /// of the future will include the appropriate context information.
    ///
    /// # Parameters
    ///
    /// * `context` - The log context to attach to this future
    ///
    /// # Returns
    ///
    /// A wrapped future that manages the context during polling
    fn in_log_context(self, context: LogContext) -> LogContextFuture<Self>;
}

impl<F> FutureExt for F
where
    F: Future,
{
    fn in_log_context(self, context: LogContext) -> LogContextFuture<Self> {
        LogContextFuture {
            inner: self,
            properties: Some(context.0),
        }
    }
}

/// A future wrapper that propagates log context.
///
/// This type wraps any future and manages the lifecycle of a log context during
/// the future's execution. When the future is polled, the context is automatically
/// pushed onto the context stack, and when the poll ends, the context is popped off again.
///
/// This ensures that any log records emitted during the execution of the future will
/// include the appropriate context information, even across `.await` points and
/// tasks that are scheduled across different threads or executors.
///
/// Users typically don't need to create this type directly; instead, use the
/// [`in_log_context`](FutureExt::in_log_context) method from the [`FutureExt`] trait.
#[pin_project]
#[derive(Debug)]
pub struct LogContextFuture<F> {
    #[pin]
    inner: F,
    properties: Option<ContextProperties>,
}

impl<F> Future for LogContextFuture<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        // Push context onto the stack before polling the inner future
        CONTEXT_STACK.with(|stack| stack.push(this.properties.take().unwrap()));

        // Poll the inner future with the context active
        let result = this.inner.poll(cx);

        // Pop the context from the stack and store it back in properties
        // This ensures the context is preserved across multiple poll calls
        this.properties
            .replace(CONTEXT_STACK.with(|stack| stack.pop().unwrap()));

        result
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::FutureExt;
    use crate::{ContextValue, LogContext, stack::CONTEXT_STACK};

    fn get_property(idx: usize) -> Option<String> {
        CONTEXT_STACK.with(|stack| {
            let top = stack.top();
            dbg!(&top);
            top.map(|properties| properties.properties[idx].1.to_string())
        })
    }

    async fn check_nested_different_contexts(answer: u32) {
        let context = LogContext::new().record("answer", answer);

        async {
            tokio::task::yield_now().await;

            async {
                tokio::task::yield_now().await;
                assert_eq!(get_property(0), Some("None".to_string()));
            }
            .in_log_context(LogContext::new().record("answer", ContextValue::null()))
            .await;

            tokio::task::yield_now().await;
            assert_eq!(get_property(0), Some(answer.to_string()));
        }
        .in_log_context(context)
        .await;

        assert_eq!(get_property(0), None);
    }

    #[tokio::test]
    async fn test_future_with_context() {
        let context = LogContext::new().record("answer", 42);

        async {
            tokio::task::yield_now().await;
            assert_eq!(get_property(0), Some("42".to_string()));
        }
        .in_log_context(context)
        .await;

        assert_eq!(get_property(0), None);
    }

    #[tokio::test]
    async fn test_nested_future_with_common_context() {
        let context = LogContext::new().record("answer", 42);

        async {
            tokio::task::yield_now().await;

            async {
                tokio::task::yield_now().await;
                assert_eq!(get_property(0), Some("42".to_string()));
            }
            .await;

            assert_eq!(get_property(0), Some("42".to_string()));
        }
        .in_log_context(context)
        .await;

        assert_eq!(get_property(0), None);
    }

    #[tokio::test]
    async fn test_nested_future_with_different_contexts() {
        check_nested_different_contexts(42).await;
    }

    #[tokio::test]
    async fn test_join_multiple_tasks_single_thread() {
        let tasks = (0..128).map(check_nested_different_contexts);
        futures_util::future::join_all(tasks).await;
    }

    #[tokio::test]
    async fn test_join_multiple_tasks_multi_thread() {
        let handles = (0..64).map(|i| {
            tokio::spawn(futures_util::future::join_all(
                (0..128).map(|j| check_nested_different_contexts(j * i)),
            ))
        });

        let results = futures_util::future::join_all(handles).await;
        for result in results {
            result.unwrap();
        }
    }
}
