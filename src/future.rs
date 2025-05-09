use std::task::Poll;

use pin_project::pin_project;

use crate::LogContext;

pub trait FutureExt: Future + Sized {
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

#[cfg(test)]
mod tests {
    use std::panic::AssertUnwindSafe;

    use futures_util::FutureExt as _;
    use pretty_assertions::assert_eq;

    use super::FutureExt;
    use crate::{stack::CONTEXT_STACK, ContextValue, LogContext};

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
