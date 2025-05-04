use crate::{
    ContextValue, StaticCowStr,
    stack::{CONTEXT_STACK, ContextProperties},
};

pub struct LogContext(pub(crate) ContextProperties);

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
