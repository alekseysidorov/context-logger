//! Flexible values for structured logging contexts.
//!
//! This module provides the [`ContextValue`] type, which serves as a container
//! for various kinds of data that can be associated with log entries.

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
/// # Type Conversions
///
/// `ContextValue` provides automatic conversions from many primitive types:
/// * Strings (`&str`, `String`)
/// * Numeric types (`i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `i128`, `u128`, `f64`)
/// * Boolean (`bool`)
/// * Characters (`char`)
///
/// For more complex types, use the appropriate factory method:
/// * [`ContextValue::serde`] for types implementing `serde::Serialize`
/// * [`ContextValue::display`] for types implementing `std::fmt::Display`
/// * [`ContextValue::debug`] for types implementing `std::fmt::Debug`
/// * [`ContextValue::error`] for types implementing `std::error::Error`
///
/// # Examples
///
/// ```
/// use context_logger::{ContextValue, LogContext};
/// use log::info;
///
/// // Simple primitive values
/// let _guard = LogContext::new()
///     .record("user_id", "user-123")  // &str -> String
///     .record("age", 30)              // i32 -> i64
///     .record("is_admin", true)       // bool
///     .record("score", 98.6)          // f64
///     .enter();
///
/// // Custom complex values
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct User {
///     name: String,
///     role: String,
/// }
///
/// let user = User {
///     name: "Alice".to_string(),
///     role: "Admin".to_string(),
/// };
///
/// let _guard = LogContext::new()
///     .record("user", ContextValue::serde(user))
///     .record("operation", ContextValue::display("login"))
///     .record("details", ContextValue::debug(vec![1, 2, 3]))
///     .enter();
///
/// // Handling errors
/// let result: Result<(), std::io::Error> = std::fs::File::open("missing.txt").map(|_| ());
/// if let Err(e) = result {
///     let _guard = LogContext::new()
///         .record("error", ContextValue::error(e))
///         .enter();
///     info!("Operation failed");
/// }
/// ```
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
    /// Creates a null context value.
    ///
    /// This represents the absence of a value and will be rendered as "null"
    /// in most log formatters.
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{ContextValue, LogContext};
    ///
    /// let ctx = LogContext::new()
    ///     .record("optional_value", ContextValue::null());
    /// ```
    #[allow(clippy::must_use_candidate)]
    pub fn null() -> Self {
        ContextValueInner::Null.into()
    }

    /// Creates a context value from a type that implements [`serde::Serialize`].
    ///
    /// This is particularly useful for complex data structures that need to be
    /// serialized in log output. The value will be serialized when the log record
    /// is processed.
    ///
    /// # Parameters
    ///
    /// * `value` - Any value that implements `serde::Serialize`
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{ContextValue, LogContext};
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct User {
    ///     id: String,
    ///     name: String,
    ///     roles: Vec<String>,
    /// }
    ///
    /// let user = User {
    ///     id: "user-123".to_string(),
    ///     name: "Alice".to_string(),
    ///     roles: vec!["admin".to_string(), "user".to_string()],
    /// };
    ///
    /// let ctx = LogContext::new()
    ///     .record("user", ContextValue::serde(user));
    /// ```
    pub fn serde<S>(value: S) -> Self
    where
        S: serde::Serialize + Send + Sync + 'static,
    {
        let value = Box::new(value);
        ContextValueInner::Serde(value).into()
    }

    /// Creates a context value from a type that implements [`std::fmt::Display`].
    ///
    /// This method is useful when you want the value to be rendered using its
    /// human-readable string representation.
    ///
    /// # Parameters
    ///
    /// * `value` - Any value that implements `std::fmt::Display`
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{ContextValue, LogContext};
    /// use std::time::{Duration, Instant};
    ///
    /// struct ElapsedTime(Instant);
    ///
    /// impl std::fmt::Display for ElapsedTime {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "{}ms", self.0.elapsed().as_millis())
    ///     }
    /// }
    ///
    /// let start_time = ElapsedTime(Instant::now());
    /// // Do some work...
    ///
    /// let ctx = LogContext::new()
    ///     .record("elapsed", ContextValue::display(start_time));
    /// ```
    pub fn display<T>(value: T) -> Self
    where
        T: std::fmt::Display + Send + Sync + 'static,
    {
        let value = Box::new(value);
        ContextValueInner::Display(value).into()
    }

    /// Creates a context value from a type that implements [`std::fmt::Debug`].
    ///
    /// Use this method when you want to include debug representations of types
    /// that may not implement other traits like `Display` or `Serialize`.
    ///
    /// # Parameters
    ///
    /// * `value` - Any value that implements `std::fmt::Debug`
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{ContextValue, LogContext};
    ///
    /// #[derive(Debug)]
    /// struct CustomType {
    ///     id: u64,
    ///     data: Vec<u8>,
    /// }
    ///
    /// let custom = CustomType {
    ///     id: 42,
    ///     data: vec![1, 2, 3, 4],
    /// };
    ///
    /// let ctx = LogContext::new()
    ///     .record("debug_value", ContextValue::debug(custom));
    /// ```
    pub fn debug<T>(value: T) -> Self
    where
        T: std::fmt::Debug + Send + Sync + 'static,
    {
        let value = Box::new(value);
        ContextValueInner::Debug(value).into()
    }

    /// Creates a context value from a type that implements [`std::error::Error`].
    ///
    /// This is particularly useful for including error information in log context.
    /// Many log formatters will include special handling for error values, potentially
    /// including the error message, cause chain, and other details.
    ///
    /// # Parameters
    ///
    /// * `value` - Any value that implements `std::error::Error`
    ///
    /// # Examples
    ///
    /// ```
    /// use context_logger::{ContextValue, LogContext};
    /// use std::io;
    /// use log::error;
    ///
    /// fn process_file() {
    ///     let result = std::fs::File::open("missing.txt");
    ///     
    ///     if let Err(e) = result {
    ///         // Add the error to the log context
    ///         let _guard = LogContext::new()
    ///             .record("error", ContextValue::error(e))
    ///             .enter();
    ///             
    ///         error!("Failed to open file");
    ///     }
    /// }
    /// ```
    pub fn error<T>(value: T) -> Self
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        let value = Box::new(value);
        ContextValueInner::Error(value).into()
    }

    /// Converts this context value to a [`log::kv::Value`] for use with the `log` crate.
    ///
    /// This method is primarily used internally by the `ContextLogger` to convert
    /// context values to the format expected by the `log` crate's structured logging
    /// system. It's generally not necessary to call this method directly.
    ///
    /// # Returns
    ///
    /// A `log::kv::Value` representing this context value.
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
