use std::cell::{Ref, RefCell, RefMut};

use crate::{ContextValue, StaticCowStr};

thread_local! {
    pub static CONTEXT_STACK: ContextStack = const { ContextStack::new() };
}

pub type ContextProperties = Vec<(StaticCowStr, ContextValue)>;

#[derive(Debug)]
pub struct ContextStack {
    inner: RefCell<Vec<ContextProperties>>,
}

impl ContextStack {
    pub const fn new() -> Self {
        ContextStack {
            inner: RefCell::new(Vec::new()),
        }
    }

    pub fn push(&self, properties: ContextProperties) {
        self.inner.borrow_mut().push(properties);
    }

    pub fn pop(&self) -> Option<ContextProperties> {
        self.inner.borrow_mut().pop()
    }

    pub fn top(&self) -> Option<Ref<ContextProperties>> {
        let inner = self.inner.borrow();
        if inner.is_empty() {
            None
        } else {
            Some(Ref::map(inner, |inner| inner.last().unwrap()))
        }
    }

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
