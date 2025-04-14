use crate::{
    context_properties::{ContextProperties, StaticCowStr},
    context_stack::ContextStack,
    ContextValue,
};

thread_local! {
    pub static CONTEXT_STACK: ContextStack = const { ContextStack::new() };
}

pub struct Context {
    properties: ContextProperties,
}

impl Context {
    pub const fn new() -> Self {
        Context {
            properties: ContextProperties::new(),
        }
    }

    pub fn with_property(
        mut self,
        key: impl Into<StaticCowStr>,
        value: impl Into<ContextValue>,
    ) -> Self {
        self.properties = self.properties.with_property(key.into(), value.into());
        self
    }

    pub fn enter(self) -> ContextGuard {
        ContextGuard::enter(self)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[non_exhaustive]
pub struct ContextGuard {}

impl ContextGuard {
    fn enter(context: Context) -> Self {
        CONTEXT_STACK.with(|stack| stack.push(context.properties));
        Self {}
    }
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        CONTEXT_STACK.with(|stack| stack.pop());
    }
}
