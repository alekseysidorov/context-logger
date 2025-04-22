use std::{pin::Pin, task::Poll};

use pin_project::pin_project;

use crate::{
    context_properties::{ContextProperties, StaticCowStr},
    context_stack::ContextStack,
    ContextValue,
};

thread_local! {
    pub static CONTEXT_STACK: ContextStack = const { ContextStack::new() };
}

pub struct LogContext {
    properties: ContextProperties,
}

impl LogContext {
    pub const fn new() -> Self {
        Self {
            properties: ContextProperties::new(),
        }
    }

    pub fn with_property(
        mut self,
        key: impl Into<StaticCowStr>,
        value: impl Into<ContextValue>,
    ) -> Self {
        let property = (key.into(), value.into());
        self.properties.0.push(property);
        self
    }

    pub fn add_property(key: impl Into<StaticCowStr>, value: impl Into<ContextValue>) {
        let property = (key.into(), value.into());
        CONTEXT_STACK.with(|stack| {
            if let Some(mut properties) = stack.current_properties_mut() {
                properties.0.push(property);
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
        CONTEXT_STACK.with(|stack| stack.push(context.properties));
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
            properties: Some(context.properties),
        }
    }
}
