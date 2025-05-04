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
