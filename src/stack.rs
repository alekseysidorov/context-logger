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
    pub fn records(&self) -> impl ExactSizeIterator<Item = LogRecordRef<'_>> + Clone {
        self.0.local.iter()
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

#[cfg(test)]
impl ScopeFrame {
    // /// Returns the first record with the given key, or `None` if not found.
    // ///
    // /// Performs a linear scan over all records in the frame — O(n).
    // pub fn find(&self, key: &str) -> Option<&crate::LogValue> {
    //     self.local
    //         .iter()
    //         .find(|r| r.key() == key)
    //         .map(crate::record::LogRecord::value)
    // }

    // pub fn is_empty(&self) -> bool {
    //     self.local.is_empty()
    // }
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
    pub fn push(&self, frame: impl Into<ScopeFrame>) {
        let frame = frame.into();
        self.inner.borrow_mut().push(frame);
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
    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }
}
