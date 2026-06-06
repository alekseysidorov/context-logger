//! Internal thread-local stack for maintaining log scopes.
//!
//! The stack is used by both the synchronous and asynchronous log
//! context propagation mechanisms.

use std::{
    borrow::Cow,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
};

use crate::record::LogRecord;

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
#[derive(Debug, Clone)]
pub struct ScopeFrame {
    /// Records attached at this scope level.
    local: HashMap<Cow<'static, str>, LogRecord>,
}

/// A stack of scope frames, one per active [`crate::LogScope`].
#[derive(Debug)]
pub struct ScopeStack {
    inner: RefCell<Vec<ScopeFrame>>,
}

impl ScopeFrame {
    pub fn new() -> Self {
        Self {
            local: HashMap::new(),
        }
    }

    pub fn push(&mut self, record: impl Into<LogRecord>) {
        let rec: LogRecord = record.into();
        let key: Cow<'static, str> = Cow::Owned(rec.key().to_string());
        self.local.insert(key, rec);
    }

    pub fn records(&self) -> impl ExactSizeIterator<Item = &LogRecord> + Clone {
        self.local.values()
    }
}

#[cfg(test)]
impl ScopeFrame {
    /// Returns the first record with the given key, or `None` if not found.
    ///
    /// Direct lookup to the records in the frame : O(1).
    pub fn find(&self, key: &str) -> Option<&crate::LogValue> {
        self.local.get(key).map(|r| r.value())
    }

    pub fn is_empty(&self) -> bool {
        self.local.is_empty()
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
    pub fn push(&self, frame: ScopeFrame) {
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
