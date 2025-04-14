use std::cell::{Ref, RefCell};

use crate::context_properties::ContextProperties;

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

    pub fn current_properties(&self) -> Option<Ref<ContextProperties>> {
        let inner = self.inner.borrow();
        if inner.is_empty() {
            None
        } else {
            Some(Ref::map(inner, |inner| inner.last().unwrap()))
        }
    }
}
