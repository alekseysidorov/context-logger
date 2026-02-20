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

/// Represents a type of value that can be stored in the log context.
///
/// The `ContextValue` type is a flexible container designed to hold various kinds of data
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
/// use context_logger::ContextValue;
///
/// let value = ContextValue::display("example string");
/// let number = ContextValue::from(42);
/// let debug_value = ContextValue::debug(vec![1, 2, 3]);
/// ```
#[derive(Clone)]
pub struct ContextValue(ContextValueInner);

#[derive(Clone)]
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
    Debug(Arc<dyn std::fmt::Debug + Send + Sync + 'static>),
    Display(Arc<dyn std::fmt::Display + Send + Sync + 'static>),
    Error(Arc<dyn std::error::Error + Send + Sync + 'static>),
    Serde(SerdeArc),
}

impl From<ContextValueInner> for ContextValue {
    fn from(inner: ContextValueInner) -> Self {
        Self(inner)
    }
}

impl ContextValue {
    /// Creates a null context value.
    #[allow(clippy::must_use_candidate)]
    pub fn null() -> Self {
        ContextValueInner::Null.into()
    }

    /// Creates a context value from a [`serde::Serialize`].
    pub fn serde<S>(value: S) -> Self
    where
        S: serde::Serialize + Send + Sync + 'static,
    {
        ContextValueInner::Serde(SerdeArc::new(value)).into()
    }

    /// Creates a context value from a [`std::fmt::Display`].
    pub fn display<T>(value: T) -> Self
    where
        T: std::fmt::Display + Send + Sync + 'static,
    {
        ContextValueInner::Display(Arc::new(value)).into()
    }

    /// Creates a context value from a [`std::fmt::Debug`].
    pub fn debug<T>(value: T) -> Self
    where
        T: std::fmt::Debug + Send + Sync + 'static,
    {
        ContextValueInner::Debug(Arc::new(value)).into()
    }

    /// Creates a context value from a [`std::error::Error`].
    pub fn error<T>(value: T) -> Self
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        ContextValueInner::Error(Arc::new(value)).into()
    }

    /// Represents a context value that can be used with the [`log`] crate.
    #[must_use]
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
            ContextValueInner::Display(value) => log::kv::Value::from_dyn_display(&**value),
            ContextValueInner::Debug(value) => log::kv::Value::from_dyn_debug(&**value),
            ContextValueInner::Error(value) => log::kv::Value::from_dyn_error(&**value),
            ContextValueInner::Serde(value) => log::kv::Value::from_serde(value),
        }
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
