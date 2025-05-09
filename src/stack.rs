//! Thread-local stack for maintaining logging context.
//!
//! This module provides the internal context stack implementation that powers
//! the context propagation features. It maintains a thread-local stack of
//! context properties that are automatically included in log records.
//!
//! The stack is used by both the synchronous (guard-based) and asynchronous
//! (future-based) context propagation mechanisms.

use std::cell::{Ref, RefCell, RefMut};

use crate::{ContextValue, StaticCowStr};

/// Thread-local stack of context properties.
///
/// Each thread has its own independent stack of context properties,
/// ensuring thread safety without synchronization overhead.
thread_local! {
    pub static CONTEXT_STACK: ContextStack = const { ContextStack::new() };
}

/// A stack of context properties that can be pushed and popped.
///
/// This structure maintains a stack of context properties within a thread.
/// When logging occurs, the properties from the top of the stack are
/// automatically included in log records.
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

    /// Pushes a set of context properties onto the stack.
    ///
    /// This method is used when entering a new context scope, either
    /// through the guard-based API or the future-based API.
    pub fn push(&self, properties: ContextProperties) {
        self.inner.borrow_mut().push(properties);
    }

    /// Pops the top set of context properties from the stack.
    ///
    /// This method is used when exiting a context scope, either when a
    /// guard is dropped or when a future completes a poll operation.
    ///
    /// # Returns
    ///
    /// The properties that were at the top of the stack, or `None` if the stack is empty.
    pub fn pop(&self) -> Option<ContextProperties> {
        self.inner.borrow_mut().pop()
    }

    /// Gets an immutable reference to the top set of context properties.
    ///
    /// This method is used when adding context properties to log records
    /// without modifying the stack.
    ///
    /// # Returns
    ///
    /// A reference to the properties at the top of the stack, or `None` if the stack is empty.
    pub fn top(&self) -> Option<Ref<ContextProperties>> {
        let inner = self.inner.borrow();
        if inner.is_empty() {
            None
        } else {
            Some(Ref::map(inner, |inner| inner.last().unwrap()))
        }
    }

    /// Gets a mutable reference to the top set of context properties.
    ///
    /// This method is used when adding properties to the current context
    /// without pushing or popping the stack, such as with `LogContext::add_record`.
    ///
    /// # Returns
    ///
    /// A mutable reference to the properties at the top of the stack, or `None` if the stack is empty.
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

/// A collection of key-value properties that make up a logging context.
///
/// This structure stores the actual key-value pairs that will be included
/// in log records. Each key is a static string (or static reference), and
/// each value is a `ContextValue` that can hold various types of data.
#[derive(Default, Debug)]
pub struct ContextProperties {
    pub properties: Vec<(StaticCowStr, ContextValue)>,
}

impl<'a> IntoIterator for &'a ContextProperties {
    type Item = &'a (StaticCowStr, ContextValue);
    type IntoIter = std::slice::Iter<'a, (StaticCowStr, ContextValue)>;

    fn into_iter(self) -> Self::IntoIter {
        self.properties.iter()
    }
}

impl ContextProperties {
    /// Creates a new, empty set of context properties.
    pub const fn new() -> Self {
        ContextProperties {
            properties: Vec::new(),
        }
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
