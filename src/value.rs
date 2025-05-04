pub struct ContextValue(ContextValueInner);

enum ContextValueInner {
    Null,
    String(String),
    Bool(bool),
    Char(char),
    I64(i64),
    U64(u64),
    F64(f64),
    I128(i128),
    U128(u128),
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
            ContextValueInner::Bool(b) => log::kv::Value::from(*b),
            ContextValueInner::Char(c) => log::kv::Value::from(*c),
            ContextValueInner::I64(i) => log::kv::Value::from(*i),
            ContextValueInner::U64(u) => log::kv::Value::from(*u),
            ContextValueInner::F64(f) => log::kv::Value::from(*f),
            ContextValueInner::I128(i) => log::kv::Value::from(*i),
            ContextValueInner::U128(u) => log::kv::Value::from(*u),
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

macro_rules! impl_context_value_from_primitive {
    ($($ty:ty => $arm:ident),*) => {
        $(
            impl From<$ty> for ContextValue {
                fn from(value: $ty) -> Self {
                    ContextValue(ContextValueInner::$arm(value.into()))
                }
            }
        )*
    };
}

impl_context_value_from_primitive!(
    bool => Bool,
    char => Char,
    String => String,
    i8 => I64,
    i16 => I64,
    i32 => I64,
    i64 => I64,
    u8 => U64,
    u16 => U64,
    u32 => U64,
    u64 => U64,
    f64 => F64,
    i128 => I128,
    u128 => U128
);

impl std::fmt::Display for ContextValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_log_value().fmt(f)
    }
}

impl std::fmt::Debug for ContextValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_log_value().fmt(f)
    }
}
