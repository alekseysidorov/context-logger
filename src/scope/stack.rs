//! Internal thread-local stack for maintaining log scopes.
//!
//! The stack is used by both the synchronous and asynchronous log
//! context propagation mechanisms.

use std::cell::{Ref, RefCell, RefMut};

use crate::{LogContext, records::LogRecordRef};

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

    /// Returns an iterator over the all records in this scope frame.
    pub fn records(&self) -> impl Iterator<Item = LogRecordRef<'_>> + Clone {
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

    /// Pushes a new scope frame onto the stack.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn push(&self, mut context: LogContext) {
        // Take the inherited records from the top frame, if stack is not empty.
        // And then merge them into the new frame's inherited records.
        //
        // This ensures that child scopes inherit records from their parent scopes.
        context.inherited.extend(
            self.top()
                .map(|top| top.0.inherited.clone())
                .unwrap_or_default(),
        );

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
    use super::*;
    use crate::LogRecords;

    fn record_to_string(record: LogRecordRef<'_>) -> (&str, String) {
        (record.0.as_ref(), record.1.to_string())
    }

    #[test]
    fn test_scope_frame_records_with_inherited() {
        let frame = ScopeFrame(LogContext {
            local: LogRecords::new().field("name", "bob"),
            inherited: LogRecords::new().field("tag", 42),
        });

        let records: Vec<_> = frame.records().map(record_to_string).collect();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0], ("tag", "42".to_string()));
    }
}
