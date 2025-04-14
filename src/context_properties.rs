use std::borrow::Cow;

pub type StaticCowStr = Cow<'static, str>;

pub struct ContextValue(ContextValueInner);

enum ContextValueInner {
    String(StaticCowStr),
    Debug(Box<dyn std::fmt::Debug>),
    Serde(Box<dyn erased_serde::Serialize>),
}

#[derive(Default)]
pub struct ContextProperties(pub Vec<(StaticCowStr, ContextValue)>);

impl<'a> IntoIterator for &'a ContextProperties {
    type Item = &'a (StaticCowStr, ContextValue);
    type IntoIter = std::slice::Iter<'a, (StaticCowStr, ContextValue)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl ContextProperties {
    pub const fn new() -> Self {
        ContextProperties(Vec::new())
    }

    pub fn with_property(mut self, key: StaticCowStr, value: ContextValue) -> Self {
        self.0.push((key, value));
        self
    }
}

impl ContextValue {
    pub fn serde<S>(value: S) -> Self
    where
        S: serde::Serialize + 'static,
    {
        ContextValue(ContextValueInner::Serde(Box::new(value)))
    }

    pub fn debug<T>(value: T) -> Self
    where
        T: std::fmt::Debug + 'static,
    {
        ContextValue(ContextValueInner::Debug(Box::new(value)))
    }

    pub fn as_log_value(&self) -> log::kv::Value<'_> {
        match &self.0 {
            ContextValueInner::String(s) => log::kv::Value::from(s.as_ref()),
            ContextValueInner::Debug(value) => log::kv::Value::from_dyn_debug(value),
            ContextValueInner::Serde(value) => log::kv::Value::from_serde(value),
        }
    }
}

impl From<&str> for ContextValue {
    fn from(value: &str) -> Self {
        ContextValue(ContextValueInner::String(Cow::Owned(value.to_owned())))
    }
}
