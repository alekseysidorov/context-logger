use crate::{value::ContextValue, StaticCowStr};

#[derive(Default)]
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
    pub const fn new() -> Self {
        ContextProperties {
            properties: Vec::new(),
        }
    }
}
