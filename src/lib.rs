//! # Overview
//!
#![doc = include_utils::include_md!("README.md:description")]
//!
//! ## How it works
//!
//! When a log record flows through `ContextLogger`, fields are resolved in this order:
//!
//! 1. Static default fields (e.g. `service`, `version`)
//! 2. Computed default fields (e.g. `timestamp`, `level`)
//! 3. Inherited fields from all parent scopes
//! 4. Local fields of the active scope
//!
//! These fields are merged into the [`log::Record`]'s key-value store — "last write wins"
//! for duplicate keys.
//!
//! ## Concepts
//!
//! The `log` crate models structured logging through a [`log::kv::Source`] — a push-based
//! iterator over key-value pairs attached to a [`log::Record`]. Context logger
//! adds an additional layer that injects scoped fields into that source.
//!
//! - **[`log::Record`]** — the log entry produced by `log::info!`, `log::warn!`,
//!   etc. Each record carries a [`key_values()`] source that consumers visit
//!   to extract structured attributes.
//! - **Field** — a key-value pair (`&str → LogValue`) attached to a log record's
//!   `Source`. Fields carry scoped context like `request_id` or `user_id`.
//! - **[`LogFields`]** — a collection of fields that implements the extension logic
//!   for the log record's source. When the record flows through [`ContextLogger`],
//!   its fields are merged into the record's own `Source`.
//! - **[`LogContext`]** — the blueprint for fields. It splits fields into
//!    [`local`](LogContext::local) and [`inherited`](LogContext::inherited)
//!   categories with different propagation semantics.
//! - **[`LogScope`] guard** — activates a `LogContext`, pushing its fields onto
//!   the thread-local scope stack. Fields are resolved when a log record flows through
//!   the logger.
//!
//! The scope stack is thread-local: each thread maintains its own independent stack
//! ensuring thread-safety without expensive synchronization.
//!
//! ## Compatibility
//!
//! `ContextLogger` wraps any type implementing [`log::Log`]. For structured key-value
//! output, pair it with a logger that supports the `kv` feature (e.g. [`env_logger`]
//! with `features = ["kv"]` or  [`log4rs`]).
//!
//! [`log::Log`]: https://docs.rs/log/latest/log/trait.Log.html
//!
//! ## Basic example
//!
#![doc = include_utils::include_md!("README.md:basic_example")]
//!
//! ## Async Context Propagation
//!
#![doc = include_utils::include_md!("README.md:async_example")]
//!
//! [`env_logger`]: https://docs.rs/env_logger/latest/env_logger
//! [`log4rs`]: https://docs.rs/log4rs/latest/log4rs

use std::{borrow::Cow, collections::HashMap};

use crate::fields::LogFieldRef;

mod context;
mod fields;
pub mod future;
mod scope;
mod value;

type LogValueFn = Box<dyn Fn(&log::Record) -> LogValue + Send + Sync>;

pub use self::{
    context::LogContext,
    fields::LogFields,
    future::FutureExt,
    scope::{LogContextExt, LogScope},
    value::LogValue,
};

/// A logger wrapper that enhances [`log::Record`] with scoped fields.
///
/// `ContextLogger` wraps an existing logging implementation and merges additional
/// fields from the current scope stack into each [`log::Record`]. These fields
/// are taken from the scope stack managed by [`LogScope`].
///
/// See the [crate-level docs](index.html) for an overview and examples.
///
/// See [`LogContext`] for more information on how to create and manage scope fields.
pub struct ContextLogger {
    inner: Box<dyn log::Log>,
    default_fields: LogFields,
    dynamic_default_fields: HashMap<Cow<'static, str>, LogValueFn>,
}

impl ContextLogger {
    /// Creates a new [`ContextLogger`] that wraps the given logging implementation.
    ///
    /// The inner logger will receive log records enhanced with scope fields
    /// from the current scope stack.
    pub fn new<L>(inner: L) -> Self
    where
        L: log::Log + 'static,
    {
        Self {
            inner: Box::new(inner),
            default_fields: LogFields::new(),
            dynamic_default_fields: HashMap::new(),
        }
    }

    /// Initializes the global logger with the context logger.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur before initialization will be ignored.
    ///
    /// # Panics
    ///
    /// Panics if a logger has already been set.
    pub fn init(self, max_level: log::LevelFilter) {
        self.try_init(max_level)
            .expect("ContextLogger::init should not be called after logger initialization");
    }

    /// Initializes the global logger with the context logger.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur before initialization will be ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if a logger has already been set.
    pub fn try_init(self, max_level: log::LevelFilter) -> Result<(), log::SetLoggerError> {
        log::set_max_level(max_level);
        log::set_boxed_logger(Box::new(self))
    }

    /// Adds a default field that will be included in every [`log::Record`].
    ///
    /// Default fields are automatically merged into each log record, regardless of
    /// the current context. They are defined when the logger is created and remain
    /// constant throughout the application's lifetime.
    ///
    /// # Behavior with Duplicate Keys
    ///
    /// When logging, default fields are added first, followed by fields from the current
    /// context. If multiple fields with the same key exist, the behavior depends on the
    /// underlying logger implementation. In most implementations, later fields with the
    /// same key will typically replace earlier ones.
    ///
    /// # Example
    ///
    /// ```
    /// use log::{info, LevelFilter};
    /// use context_logger::{ContextLogger, LogContext, LogScope};
    ///
    /// // Create a logger with default fields
    /// let logger = ContextLogger::new(env_logger::builder()
    ///       .filter_level(log::LevelFilter::Info)
    ///       .build())
    ///       .with_default_field("service", "api")
    ///     .with_default_field("version", "1.0.0");
    /// // Initialize it
    /// logger.init(LevelFilter::Info);
    ///
    /// info!("Processing request"); // Will include service="api", version="1.0.0"
    /// ```
    #[must_use]
    pub fn with_default_field(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<LogValue>,
    ) -> Self {
        self.default_fields.insert(key, value);
        self
    }

    /// Adds a dynamic default field computed by the given closure for every [`log::Record`].
    ///
    /// Like [`Self::with_default_field`], the field is merged into each log record.
    /// However, unlike the static variant, the value is *computed at log time* by invoking
    /// the provided closure with the current [`log::Record`] itself. This makes it suitable
    /// for fields whose values are not known upfront, such as timestamps or thread IDs.
    ///
    /// **Note!** *The order in which dynamic default field functions are evaluated is not guaranteed.*
    ///
    /// # Example
    ///
    /// Adding a current timestamp.
    ///
    /// ```
    /// use chrono::Utc;
    /// use log::{info, LevelFilter};
    /// use context_logger::{ContextLogger, LogValue};
    ///
    /// let logger = ContextLogger::new(env_logger::builder()
    ///       .filter_level(log::LevelFilter::Info)
    ///       .build())
    ///       .with_default_field_fn("timestamp", |_record| {
    ///          Utc::now().to_rfc3339().to_string()
    ///     });
    /// logger.init(LevelFilter::Info);
    ///
    /// info!("Hello");  // Will include timestamp="..."
    /// ```
    #[must_use]
    pub fn with_default_field_fn<V: Into<LogValue>>(
        mut self,
        key: impl Into<Cow<'static, str>>,
        f: impl Fn(&log::Record) -> V + Send + Sync + 'static,
    ) -> Self {
        self.dynamic_default_fields
            .insert(key.into(), Box::new(move |record| f(record).into()));
        self
    }
}

impl std::fmt::Debug for ContextLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextLogger").finish_non_exhaustive()
    }
}

impl log::Log for ContextLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let error = scope::stack::SCOPE_STACK.try_with(|stack| {
            let dynamic_default_fields = self
                .dynamic_default_fields
                .iter()
                .map(|(key, f)| (key, f(record)))
                .collect::<Vec<_>>();

            let default_fields = self
                .default_fields
                .iter()
                .chain(dynamic_default_fields.iter().map(|(k, v)| (*k, v)));

            // Only the top frame is read here intentionally: inherited fields from
            // outer scopes are copied into each newly entered frame on `enter()`,
            // so the top frame always contains a complete, flat view of active fields.
            if let Some(top) = stack.top() {
                self.inner.log(
                    &record
                        .to_builder()
                        .key_values(&SourceWithFields {
                            source: &record.key_values(),
                            fields: default_fields.chain(top.fields()),
                        })
                        .build(),
                );
            } else {
                self.inner.log(
                    &record
                        .to_builder()
                        .key_values(&SourceWithFields {
                            source: &record.key_values(),
                            fields: default_fields,
                        })
                        .build(),
                );
            }
        });

        if let Err(err) = error {
            // If the context stack is not available, log the original record.
            self.inner.log(record);
            // We can't use `log::error!` here because we are in the middle of logging and
            // this invocation becomes recursive.
            eprintln!("Error accessing context stack: {err}");
        }
    }

    fn flush(&self) {
        self.inner.flush();
    }
}

struct SourceWithFields<'a, I> {
    source: &'a dyn log::kv::Source,
    fields: I,
}

impl<'a, I> log::kv::Source for SourceWithFields<'a, I>
where
    I: Iterator<Item = LogFieldRef<'a>> + Clone,
{
    fn visit<'kvs>(
        &'kvs self,
        visitor: &mut dyn log::kv::VisitSource<'kvs>,
    ) -> Result<(), log::kv::Error> {
        for (key, value) in self.fields.clone() {
            visitor.visit_pair(log::kv::Key::from_str(key), value.as_log_value())?;
        }
        self.source.visit(visitor)
    }
}

mod private {
    pub trait Sealed {}

    impl<F: Future> Sealed for F {}
    impl Sealed for crate::LogContext {}
}
