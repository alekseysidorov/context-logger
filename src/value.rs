pub struct ContextValue(ContextValueInner);

enum ContextValueInner {
    Null,
    String(String),
    Debug(Box<dyn std::fmt::Debug + Send + Sync + 'static>),
    Display(Box<dyn std::fmt::Display + Send + Sync + 'static>),
    Error(Box<dyn std::error::Error + Send + Sync + 'static>),
    Serde(Box<dyn erased_serde::Serialize + Send + Sync + 'static>),
}

impl From<ContextValueInner> for ContextValue {
    fn from(inner: ContextValueInner) -> Self {
        ContextValue(inner)
    }
}

impl ContextValue {
    pub fn null() -> Self {
        ContextValueInner::Null.into()
    }

    pub fn serde<S>(value: S) -> Self
    where
        S: serde::Serialize + Send + Sync + 'static,
    {
        let value = Box::new(value);
        ContextValueInner::Serde(value).into()
    }

    pub fn display<T>(value: T) -> Self
    where
        T: std::fmt::Display + Send + Sync + 'static,
    {
        let value = Box::new(value);
        ContextValueInner::Display(value).into()
    }

    pub fn debug<T>(value: T) -> Self
    where
        T: std::fmt::Debug + Send + Sync + 'static,
    {
        let value = Box::new(value);
        ContextValueInner::Debug(value).into()
    }

    pub fn error<T>(value: T) -> Self
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        let value = Box::new(value);
        ContextValueInner::Error(value).into()
    }

    pub fn as_log_value(&self) -> log::kv::Value<'_> {
        match &self.0 {
            ContextValueInner::Null => log::kv::Value::null(),
            ContextValueInner::String(s) => log::kv::Value::from(&**s),
            ContextValueInner::Display(value) => log::kv::Value::from_dyn_display(value),
            ContextValueInner::Debug(value) => log::kv::Value::from_dyn_debug(value),
            ContextValueInner::Error(value) => log::kv::Value::from_dyn_error(&**value),
            ContextValueInner::Serde(value) => log::kv::Value::from_serde(value),
        }
    }
}

impl From<&str> for ContextValue {
    fn from(value: &str) -> Self {
        ContextValue(ContextValueInner::String(value.to_owned()))
    }
}
