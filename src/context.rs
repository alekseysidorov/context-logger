use std::{pin::Pin, task::Poll};

use pin_project::pin_project;

use crate::{properties::ContextProperties, stack::ContextStack, ContextValue, StaticCowStr};

thread_local! {
    pub static CONTEXT_STACK: ContextStack = const { ContextStack::new() };
}

pub struct LogContext(ContextProperties);

impl LogContext {
    pub const fn new() -> Self {
        Self(ContextProperties::new())
    }

    pub fn record(mut self, key: impl Into<StaticCowStr>, value: impl Into<ContextValue>) -> Self {
        let property = (key.into(), value.into());
        self.0.properties.push(property);
        self
    }

    pub fn add_record(key: impl Into<StaticCowStr>, value: impl Into<ContextValue>) {
        let property = (key.into(), value.into());

        CONTEXT_STACK.with(|stack| {
            if let Some(mut top) = stack.top_mut() {
                top.properties.push(property);
            }
        });
    }

    pub fn enter(self) -> LogContextGuard {
        LogContextGuard::enter(self)
    }
}

impl Default for LogContext {
    fn default() -> Self {
        Self::new()
    }
}

#[non_exhaustive]
pub struct LogContextGuard {}

impl LogContextGuard {
    fn enter(context: LogContext) -> Self {
        CONTEXT_STACK.with(|stack| stack.push(context.0));
        Self {}
    }
}

impl Drop for LogContextGuard {
    fn drop(&mut self) {
        CONTEXT_STACK.with(|stack| stack.pop());
    }
}

#[pin_project]
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

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        CONTEXT_STACK.with(|stack| stack.push(this.properties.take().unwrap()));
        let result = this.inner.poll(cx);
        this.properties
            .replace(CONTEXT_STACK.with(|stack| stack.pop().unwrap()));

        result
    }
}

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
