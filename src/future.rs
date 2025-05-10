//! Future types.

use std::task::Poll;

use pin_project::pin_project;

use crate::LogContext;

/// Extension trait for futures to propagate contextual logging information.
///
/// This traits adds ability to attach a [`LogContext`] for any [`Future`],
/// ensuring that logs emitted during the future's execution will include
/// the contextual properties even the future is polled across different threads.
pub trait FutureExt: Sized + private::Sealed {
    /// Attaches a log context to this future.
    ///
    /// The attached [context](LogContext) will be activated every time the instrumented
    /// future is polled.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{LogContext, FutureExt};
    /// use log::info;
    ///
    /// async fn process_user_data(user_id: u64) {
    ///     // Create a context with user information
    ///     let context = LogContext::new()
    ///         .record("user_id", user_id)
    ///         .record("operation", "process_data");
    ///
    ///     async {
    ///         info!("Starting user data processing"); // Will include context
    ///
    ///         // Do some async work...
    ///
    ///         info!("User data processing complete"); // Still includes context
    ///     }
    ///     .in_log_context(context)
    ///     .await;
    /// }
    /// ```
    fn in_log_context(self, context: LogContext) -> LogContextFuture<Self>;
}

impl<F> FutureExt for F
where
    F: Future,
{
    fn in_log_context(self, context: LogContext) -> LogContextFuture<Self> {
        LogContextFuture {
            inner: self,
            log_context: Some(context),
        }
    }
}

/// A future with an attached logging context.
///
/// This type is created by the [`FutureExt::in_log_context`].
///
/// # Note
///
/// If the wrapped future will panic, the next `poll` invocation panic unconditionally.
#[pin_project]
#[derive(Debug)]
pub struct LogContextFuture<F> {
    #[pin]
    inner: F,
    log_context: Option<LogContext>,
}

impl<F> Future for LogContextFuture<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let log_context = this
            .log_context
            .take()
            .expect("An attempt to poll panicked future");

        let guard = log_context.enter();
        let result = this.inner.poll(cx);
        this.log_context.replace(guard.exit());

        result
    }
}

mod private {
    pub trait Sealed {}

    impl<F: Future> Sealed for F {}
}

#[cfg(test)]
mod tests {
    use std::panic::AssertUnwindSafe;

    use futures_util::FutureExt as _;
    use pretty_assertions::assert_eq;

    use super::FutureExt;
    use crate::{ContextValue, LogContext, stack::CONTEXT_STACK};

    fn get_property(idx: usize) -> Option<String> {
        CONTEXT_STACK.with(|stack| {
            let top = stack.top();
            top.map(|properties| properties[idx].1.to_string())
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
    async fn test_panicked_future() {
        let context = LogContext::new().record("answer", 42);

        AssertUnwindSafe(
            async {
                tokio::task::yield_now().await;
                panic!("Goodbye cruel world");
            }
            .in_log_context(context),
        )
        .catch_unwind()
        .await
        .unwrap_err();

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
