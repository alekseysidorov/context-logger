//! Internal thread-local stack for maintaining log scopes.
//!
//! The stack is used by both the synchronous and asynchronous log
//! context propagation mechanisms.

use std::cell::{Ref, RefCell, RefMut};

use crate::{LogContext, fields::LogFieldRef};

thread_local! {
    /// Thread-local stack for maintaining log scopes.
    ///
    /// Each thread has its own independent stack ensuring thread-safety without
    /// expensive synchronization.
    pub static SCOPE_STACK: ScopeStack = const { ScopeStack::new() };
}

/// A single frame in the thread-local [`ScopeStack`].
///
/// Pushed when a scope is entered and popped when its guard is dropped.
#[derive(Debug, Clone, Default)]
pub struct ScopeFrame(pub LogContext);

/// A stack of scope frames, one per active [`crate::LogScope`].
#[derive(Debug)]
pub struct ScopeStack {
    inner: RefCell<Vec<ScopeFrame>>,
}

impl ScopeFrame {
    pub fn new() -> Self {
        Self(LogContext::new())
    }

    /// Returns an iterator over all log fields in this scope frame.
    ///
    /// Inherited fields come first, followed by local fields. This allows local
    /// fields to shadow inherited ones when a consumer resolves duplicate keys
    /// using "last write wins" semantics.
    pub fn fields(&self) -> impl Iterator<Item = LogFieldRef<'_>> + Clone {
        self.0.inherited.iter().chain(self.0.local.iter())
    }
}

impl From<LogContext> for ScopeFrame {
    fn from(context: LogContext) -> Self {
        Self(context)
    }
}

impl From<ScopeFrame> for LogContext {
    fn from(frame: ScopeFrame) -> Self {
        frame.0
    }
}

impl ScopeStack {
    /// Creates a new, empty scope stack.
    pub const fn new() -> Self {
        Self {
            inner: RefCell::new(Vec::new()),
        }
    }

    /// Pushes a new scope frame onto the stack, merging inherited fields from
    /// the current top frame into the new context's inherited fields.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn push(&self, mut context: LogContext) {
        // Merge inherited fields from the parent frame into the child context.
        // Parent inherited fields are applied first, then child inherited fields
        // so child scopes can shadow inherited keys from their parent.
        let mut inherited = self
            .top()
            .map(|top| top.0.inherited.clone())
            .unwrap_or_default();
        inherited.merge_with(context.inherited);
        context.inherited = inherited;

        self.inner.borrow_mut().push(ScopeFrame::from(context));
    }

    /// Pops the top scope frame from the stack.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn pop(&self) -> Option<ScopeFrame> {
        self.inner.borrow_mut().pop()
    }

    /// Returns a reference to the top scope frame on the stack.
    ///
    /// # Panics
    ///
    /// If the stack is already mutably borrowed.
    pub fn top(&self) -> Option<Ref<'_, ScopeFrame>> {
        let inner = self.inner.borrow();
        if inner.is_empty() {
            None
        } else {
            Some(Ref::map(inner, |inner| inner.last().unwrap()))
        }
    }

    /// Returns a mutable reference to the top scope frame on the stack.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn top_mut(&self) -> Option<RefMut<'_, ScopeFrame>> {
        let inner = self.inner.borrow_mut();
        if inner.is_empty() {
            None
        } else {
            Some(RefMut::map(inner, |inner| inner.last_mut().unwrap()))
        }
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl ScopeStack {
    /// Returns the number of scope frames on the stack.
    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    /// Returns `true` if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::LogFields;

    fn field_to_string(entry: LogFieldRef<'_>) -> (&str, String) {
        (entry.0.as_ref(), entry.1.to_string())
    }

    #[test]
    fn test_scope_frame_fields_with_inherited() {
        let frame = ScopeFrame(LogContext {
            local: LogFields::new().with("name", "bob"),
            inherited: LogFields::new().with("tag", 42),
        });

        let fields: HashMap<_, _> = frame.fields().map(field_to_string).collect();

        assert_eq!(fields.len(), 2);
        assert_eq!(fields["tag"], "42");
    }
}
