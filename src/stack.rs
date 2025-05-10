//! Internal thread-local stack for maintaining log context.
//!
//! The stack is used by both the syncrhonous and asynchronous log
//! context propagation mechanisms.

use std::cell::{Ref, RefCell, RefMut};

use crate::{ContextValue, StaticCowStr};

thread_local! {
    /// Thread-local stack for maintaining log context.
    ///
    /// Each thread has its own independent stack ensuring thread-safety without
    /// expensive synchronization.
    pub static CONTEXT_STACK: ContextStack = const { ContextStack::new() };
}

pub type ContextProperties = Vec<(StaticCowStr, ContextValue)>;

/// A stack of context properties.
#[derive(Debug)]
pub struct ContextStack {
    inner: RefCell<Vec<ContextProperties>>,
}

impl ContextStack {
    /// Creates a new, empty context stack.
    pub const fn new() -> Self {
        ContextStack {
            inner: RefCell::new(Vec::new()),
        }
    }

    /// Pushes a new set of context properties onto the stack.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn push(&self, properties: ContextProperties) {
        self.inner.borrow_mut().push(properties);
    }

    /// Pops the top set of context properties from the stack.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn pop(&self) -> Option<ContextProperties> {
        self.inner.borrow_mut().pop()
    }

    /// Returns a reference to the top set of context properties on the stack.
    ///
    /// # Panics
    ///
    /// If the stack is already mutably borrowed.
    pub fn top(&self) -> Option<Ref<ContextProperties>> {
        let inner = self.inner.borrow();
        if inner.is_empty() {
            None
        } else {
            Some(Ref::map(inner, |inner| inner.last().unwrap()))
        }
    }

    /// Returns a mutable reference to the top set of context properties on the stack.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn top_mut(&self) -> Option<RefMut<ContextProperties>> {
        let inner = self.inner.borrow_mut();
        if inner.is_empty() {
            None
        } else {
            Some(RefMut::map(inner, |inner| inner.last_mut().unwrap()))
        }
    }
}

impl Default for ContextStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl ContextStack {
    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }
}
