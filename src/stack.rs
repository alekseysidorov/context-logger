//! Internal thread-local stack for maintaining log context.
//!
//! The stack is used by both the synchronous and asynchronous log
//! context propagation mechanisms.

use std::cell::{RefCell, RefMut};

use crate::{ContextValue, StaticCowStr};

thread_local! {
    /// Thread-local stack for maintaining log context.
    ///
    /// Each thread has its own independent stack ensuring thread-safety without
    /// expensive synchronization.
    pub static CONTEXT_STACK: ContextStack = const { ContextStack::new() };
}

pub type ContextRecords = Vec<(StaticCowStr, ContextValue)>;

/// A single frame in the context stack.
///
/// Each frame holds two sets of records:
/// - `local`: records visible only in the current context.
/// - `inherited`: records that propagate into all nested contexts.
#[derive(Debug, Clone, Default)]
pub struct ContextFrame {
    /// Records visible only in the current context.
    pub local: ContextRecords,
    /// Records that are inherited by all nested contexts.
    pub inherited: ContextRecords,
}

impl ContextFrame {
    /// Creates a new, empty context frame.
    pub const fn new() -> Self {
        Self {
            local: Vec::new(),
            inherited: Vec::new(),
        }
    }
}

/// A stack of context properties.
#[derive(Debug)]
pub struct ContextStack {
    inner: RefCell<Vec<ContextFrame>>,
}

impl ContextStack {
    /// Creates a new, empty context stack.
    pub const fn new() -> Self {
        Self {
            inner: RefCell::new(Vec::new()),
        }
    }

    /// Pushes a new context frame onto the stack.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn push(&self, frame: ContextFrame) {
        self.inner.borrow_mut().push(frame);
    }

    /// Pops the top context frame from the stack.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn pop(&self) -> Option<ContextFrame> {
        self.inner.borrow_mut().pop()
    }

    /// Returns a mutable reference to the local records of the top context frame.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn top_mut(&self) -> Option<RefMut<'_, ContextRecords>> {
        let inner = self.inner.borrow_mut();
        if inner.is_empty() {
            None
        } else {
            Some(RefMut::map(inner, |inner| {
                &mut inner.last_mut().unwrap().local
            }))
        }
    }

    /// Returns a mutable reference to the inherited records of the top context frame.
    ///
    /// # Panics
    ///
    /// If the stack is already borrowed.
    pub fn top_inherited_mut(&self) -> Option<RefMut<'_, ContextRecords>> {
        let inner = self.inner.borrow_mut();
        if inner.is_empty() {
            None
        } else {
            Some(RefMut::map(inner, |inner| {
                &mut inner.last_mut().unwrap().inherited
            }))
        }
    }

    /// Collects all records for logging.
    ///
    /// Returns the inherited records from every frame (outermost first), followed
    /// by the local records from the top frame.  This ensures that inherited
    /// properties set in an outer scope are visible inside any nested scope.
    pub fn collect_all_records(&self) -> ContextRecords {
        let inner = self.inner.borrow();
        if inner.is_empty() {
            return Vec::new();
        }
        let mut result = Vec::new();
        for frame in inner.iter() {
            result.extend_from_slice(&frame.inherited);
        }
        result.extend_from_slice(&inner.last().unwrap().local);
        result
    }
}

impl Default for ContextStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl ContextStack {
    pub fn top(&self) -> Option<std::cell::Ref<'_, ContextRecords>> {
        let inner = self.inner.borrow();
        if inner.is_empty() {
            None
        } else {
            Some(std::cell::Ref::map(inner, |inner| {
                &inner.last().unwrap().local
            }))
        }
    }

    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }
}
