use std::task::Poll;

use pin_project::pin_project;

use crate::{
    stack::{ContextProperties, CONTEXT_STACK},
    LogContext,
};

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
            properties: Some(context.0),
        }
    }
}

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

        CONTEXT_STACK.with(|stack| stack.push(this.properties.take().unwrap()));
        let result = this.inner.poll(cx);
        this.properties
            .replace(CONTEXT_STACK.with(|stack| stack.pop().unwrap()));

        result
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::FutureExt;
    use crate::{stack::CONTEXT_STACK, ContextValue, LogContext};

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
