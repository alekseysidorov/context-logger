//! Value types for the context logger.

use std::sync::Arc;

/// A sized, cloneable wrapper around `Arc<dyn erased_serde::Serialize>` that implements
/// `serde::Serialize`. This is needed because `log::kv::Value::from_serde` requires `T: Sized`,
/// but `dyn erased_serde::Serialize` is unsized.
#[derive(Clone)]
struct SerdeArc(Arc<dyn erased_serde::Serialize + Send + Sync + 'static>);

impl SerdeArc {
    fn new<T>(value: T) -> Self
    where
        T: serde::Serialize + Send + Sync + 'static,
    {
        Self(Arc::new(value))
    }
}

impl serde::Serialize for SerdeArc {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        erased_serde::serialize(&*self.0, serializer)
    }
}

/// Represents a value that can be stored in a log field.
///
/// The `LogValue` type is a flexible container designed to hold various kinds of data
/// that can be associated with a log entry. It supports primitive types, strings, and
/// more complex types such as those implementing [`std::fmt::Debug`], [`std::fmt::Display`],
/// [`std::error::Error`], or `serde::Serialize`.
///
/// This allows for rich and structured logging, enabling developers to attach meaningful
/// context to log messages.
///
/// # Examples
///
/// ```
/// use context_logger::LogValue;
///
/// let value = LogValue::display("example string");
/// let number = LogValue::from(42);
/// let debug_value = LogValue::debug(vec![1, 2, 3]);
/// ```
#[derive(Clone)]
pub struct LogValue(LogValueInner);

#[derive(Clone)]
enum LogValueInner {
    Null,
    String(String),
    Bool(bool),
    Char(char),
    I64(i64),
    U64(u64),
    F64(f64),
    I128(i128),
    U128(u128),
    Debug(Arc<dyn std::fmt::Debug + Send + Sync + 'static>),
    Display(Arc<dyn std::fmt::Display + Send + Sync + 'static>),
    Error(Arc<dyn std::error::Error + Send + Sync + 'static>),
    Serde(SerdeArc),
}

impl From<LogValueInner> for LogValue {
    fn from(inner: LogValueInner) -> Self {
        Self(inner)
    }
}

impl LogValue {
    /// Creates a null log value.
    #[allow(clippy::must_use_candidate)]
    pub fn null() -> Self {
        LogValueInner::Null.into()
    }

    /// Creates a log value from a [`serde::Serialize`].
    pub fn serde<S>(value: S) -> Self
    where
        S: serde::Serialize + Send + Sync + 'static,
    {
        LogValueInner::Serde(SerdeArc::new(value)).into()
    }

    /// Creates a log value from a [`std::fmt::Display`].
    pub fn display<T>(value: T) -> Self
    where
        T: std::fmt::Display + Send + Sync + 'static,
    {
        LogValueInner::Display(Arc::new(value)).into()
    }

    /// Creates a log value from a [`std::fmt::Debug`].
    pub fn debug<T>(value: T) -> Self
    where
        T: std::fmt::Debug + Send + Sync + 'static,
    {
        LogValueInner::Debug(Arc::new(value)).into()
    }

    /// Creates a log value from a [`std::error::Error`].
    pub fn error<T>(value: T) -> Self
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        LogValueInner::Error(Arc::new(value)).into()
    }

    /// Converts the log value to a value compatible with the [`log`] crate.
    #[must_use]
    pub fn as_log_value(&self) -> log::kv::Value<'_> {
        match &self.0 {
            LogValueInner::Null => log::kv::Value::null(),
            LogValueInner::String(s) => log::kv::Value::from(&**s),
            LogValueInner::Bool(b) => log::kv::Value::from(*b),
            LogValueInner::Char(c) => log::kv::Value::from(*c),
            LogValueInner::I64(i) => log::kv::Value::from(*i),
            LogValueInner::U64(u) => log::kv::Value::from(*u),
            LogValueInner::F64(f) => log::kv::Value::from(*f),
            LogValueInner::I128(i) => log::kv::Value::from(*i),
            LogValueInner::U128(u) => log::kv::Value::from(*u),
            LogValueInner::Display(value) => log::kv::Value::from_dyn_display(&**value),
            LogValueInner::Debug(value) => log::kv::Value::from_dyn_debug(&**value),
            LogValueInner::Error(value) => log::kv::Value::from_dyn_error(&**value),
            LogValueInner::Serde(value) => log::kv::Value::from_serde(value),
        }
    }
}

macro_rules! impl_log_value_from_primitive {
    ($($ty:ty => $arm:ident),*) => {
        $(
            impl From<$ty> for LogValue {
                fn from(value: $ty) -> Self {
                    LogValue(LogValueInner::$arm(value.into()))
                }
            }
        )*
    };
}

impl_log_value_from_primitive!(
    bool => Bool,
    char => Char,
    &str => String,
    std::borrow::Cow<'_, str> => String,
    String => String,
    i8 => I64,
    i16 => I64,
    i32 => I64,
    i64 => I64,
    u8 => U64,
    u16 => U64,
    u32 => U64,
    u64 => U64,
    f32 => F64,
    f64 => F64,
    i128 => I128,
    u128 => U128
);

impl std::fmt::Display for LogValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_log_value().fmt(f)
    }
}

impl std::fmt::Debug for LogValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_log_value().fmt(f)
    }
}
