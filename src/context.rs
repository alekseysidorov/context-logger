use crate::{
    ContextValue, StaticCowStr,
    guard::LogContextGuard,
    stack::{CONTEXT_STACK, ContextProperties},
};

#[derive(Debug)]
pub struct LogContext(pub(crate) ContextProperties);

impl LogContext {
    #[must_use]
    pub const fn new() -> Self {
        Self(ContextProperties::new())
    }

    #[must_use]
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

    #[must_use]
    pub fn enter<'a>(self) -> LogContextGuard<'a> {
        LogContextGuard::enter(self)
    }
}

impl Default for LogContext {
    fn default() -> Self {
        Self::new()
    }
}
