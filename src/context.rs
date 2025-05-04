use crate::{
    ContextValue, StaticCowStr,
    stack::{CONTEXT_STACK, ContextProperties},
};

#[derive(Debug)]
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
#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::stack::CONTEXT_STACK;

    #[test]
    fn test_log_context_guard_enter() {
        let context = LogContext::new().record("simple", 42);
        // Make sure the context stack is empty before entering the context.
        assert_eq!(CONTEXT_STACK.with(|stack| stack.is_empty()), true);

        let guard = context.enter();
        // Check that the record was added to the top context.
        assert_eq!(
            CONTEXT_STACK.with(|stack| stack.top().unwrap().properties.len()),
            1
        );

        // Check that the context stack is empty after dropping the guard.
        drop(guard);
        assert_eq!(CONTEXT_STACK.with(|stack| stack.len()), 0);
    }

    #[test]
    fn test_log_context_nested_guards() {
        let outer_context = LogContext::new().record("simple_record", "outer_value");
        assert_eq!(CONTEXT_STACK.with(|stack| stack.len()), 0);

        let outer_guard = outer_context.enter();
        assert_eq!(
            CONTEXT_STACK.with(|stack| stack.top().unwrap().properties.len()),
            1
        );

        CONTEXT_STACK.with(|stack| {
            let property = &stack.top().unwrap().properties[0];
            assert_eq!(property.0, "simple_record");
            assert_eq!(property.1.to_string(), "outer_value");
        });

        let inner_context = LogContext::new().record("simple_record", "inner_value");
        {
            let inner_guard = inner_context.enter();
            // Test log context after inner guard is entered.
            assert_eq!(CONTEXT_STACK.with(|stack| stack.len()), 2);
            CONTEXT_STACK.with(|stack| {
                let property = &stack.top().unwrap().properties[0];
                assert_eq!(property.0, "simple_record");
                assert_eq!(property.1.to_string(), "inner_value");
            });

            drop(inner_guard);
        }
        // Test log context after inner guard is dropped.
        assert_eq!(
            CONTEXT_STACK.with(|stack| stack.top().unwrap().properties.len()),
            1
        );
        CONTEXT_STACK.with(|stack| {
            let property = &stack.top().unwrap().properties[0];
            assert_eq!(property.0, "simple_record");
            assert_eq!(property.1.to_string(), "outer_value");
        });

        drop(outer_guard);
        assert_eq!(CONTEXT_STACK.with(|stack| stack.is_empty()), true);
    }

    #[test]
    fn test_log_context_multithread() {
        let first_thread_handle = std::thread::spawn(|| {
            let inner_context = LogContext::new().record("simple_record", "first_thread");
            let inner_guard = inner_context.enter();
            // Test log context after inner guard is entered.
            assert_eq!(CONTEXT_STACK.with(|stack| stack.len()), 1);
            CONTEXT_STACK.with(|stack| {
                let property = &stack.top().unwrap().properties[0];
                assert_eq!(property.0, "simple_record");
                assert_eq!(property.1.to_string(), "first_thread");
            });

            drop(inner_guard);
        });
        let second_thread_handle = std::thread::spawn(|| {
            let inner_context = LogContext::new().record("simple_record", "second_thread");
            let inner_guard = inner_context.enter();
            // Test log context after inner guard is entered.
            assert_eq!(CONTEXT_STACK.with(|stack| stack.len()), 1);
            CONTEXT_STACK.with(|stack| {
                let property = &stack.top().unwrap().properties[0];
                assert_eq!(property.0, "simple_record");
                assert_eq!(property.1.to_string(), "second_thread");
            });

            drop(inner_guard);
        });

        first_thread_handle.join().unwrap();
        second_thread_handle.join().unwrap();
    }
}
