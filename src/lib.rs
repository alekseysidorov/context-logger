//! # Overview
//!
#![doc = include_utils::include_md!("README.md:description")]
//!
//! Modern applications often need rich, structured context in logs to provide
//! insight into runtime behavior. This library simplifies the process by:
//!
//! - Adding structured context to logs without modifying the existing logging statements.
//! - Propagating log context across async boundaries.
//! - Allowing dynamic context updates.
//! - Supporting nested contexts to build hierarchical relationships.
//!
//! This library provides a wrapper around other existing logger implementations,
//! acting as a middleware layer that enriches log records with additional context before
//! passing them to the underlying logger. It works with any logger that implements the
//! standard [`Log`](log::Log) trait, making it compatible with popular logging frameworks like
//! [`env_logger`], [`log4rs`] and others.
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

use crate::records::LogRecordRef;

mod context;
pub mod future;
mod records;
mod scope;
mod value;

type LogValueFn = Box<dyn Fn(&log::Record) -> LogValue + Send + Sync>;

pub use self::{
    context::LogContext,
    future::FutureExt,
    records::LogRecords,
    scope::{LogContextExt, LogScope},
    value::LogValue,
};

/// A logger wrapper that enhances log records with scope records.
///
/// `ContextLogger` wraps an existing logging implementation and adds additional
/// scope records to log records. These records are taken from the
/// current scope stack, which is managed by [`LogScope`].
///
/// # Example
///
#[doc = include_utils::include_md!("README.md:basic_example")]
///
/// See [`LogContext`] for more information on how to create and manage scope records.
pub struct ContextLogger {
    inner: Box<dyn log::Log>,
    default_records: LogRecords,
    dynamic_default_records: HashMap<Cow<'static, str>, LogValueFn>,
}

impl ContextLogger {
    /// Creates a new [`ContextLogger`] that wraps the given logging implementation.
    ///
    /// The inner logger will receive log records enhanced with scope records
    /// from the current scope stack.
    pub fn new<L>(inner: L) -> Self
    where
        L: log::Log + 'static,
    {
        Self {
            inner: Box::new(inner),
            default_records: LogRecords::new(),
            dynamic_default_records: HashMap::new(),
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

    /// Adds a default record that will be included in all log entries.
    ///
    /// Default records are automatically added to all log entries, regardless of
    /// the current context. They are defined when the logger is created and remain
    /// constant throughout the application's lifetime.
    ///
    /// # Behavior with Duplicate Keys
    ///
    /// When logging, default records are added first, followed by records from the current
    /// context. If multiple records with the same key exist, the behavior depends on the
    /// underlying logger implementation. In most implementations, later records with the
    /// same key will typically replace earlier ones.
    ///
    /// # Example
    ///
    /// ```
    /// use log::{info, LevelFilter};
    /// use context_logger::{ContextLogger, LogContext, LogScope};
    ///
    /// // Create a logger with default records
    /// let logger = ContextLogger::new(env_logger::builder().build())
    ///     .with_default_record("service", "api")
    ///     .with_default_record("version", "1.0.0");
    /// // Initialize it
    /// logger.init(LevelFilter::Info);
    ///
    /// info!("Processing request"); // Will include service="api", version="1.0.0"
    /// ```
    #[must_use]
    pub fn with_default_record(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<LogValue>,
    ) -> Self {
        self.default_records.insert(key, value);
        self
    }

    /// Adds a dynamic default record computed by the given closure for each log entry.
    ///
    /// Like [`Self::with_default_record`], the record is included in all log entries.
    /// However, unlike the static variant, the value is *computed at log time* by invoking the
    /// provided closure with the current [`log::Record`]. This makes it suitable for fields
    /// whose values are not known upfront, such as timestamps or thread IDs.
    ///
    /// **Note!** *The order in which dynamic default record functions are evaluated is not guaranteed.*
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
    /// let logger = ContextLogger::new(env_logger::builder().build())
    ///     .with_default_record_fn("timestamp", |_record| {
    ///          Utc::now().to_rfc3339().to_string()
    ///     });
    /// logger.init(LevelFilter::Info);
    ///
    /// info!("Hello");  // Will include timestamp="..."
    /// ```
    #[must_use]
    pub fn with_default_record_fn<V: Into<LogValue>>(
        mut self,
        key: impl Into<Cow<'static, str>>,
        f: impl Fn(&log::Record) -> V + Send + Sync + 'static,
    ) -> Self {
        self.dynamic_default_records
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
            let dynamic_default_records = self
                .dynamic_default_records
                .iter()
                .map(|(key, f)| (key, f(record)))
                .collect::<Vec<_>>();
            let default_records = self
                .default_records
                .iter()
                .chain(dynamic_default_records.iter().map(|(k, v)| (*k, v)));

            // Only the top frame is read here intentionally: inherited records from
            // outer scopes are copied into each newly entered frame on `enter()`,
            // so the top frame always contains a complete, flat view of active records.
            if let Some(top) = stack.top() {
                self.inner.log(
                    &record
                        .to_builder()
                        .key_values(&SourceWithRecords {
                            source: &record.key_values(),
                            records: default_records.chain(top.records()),
                        })
                        .build(),
                );
            } else {
                self.inner.log(
                    &record
                        .to_builder()
                        .key_values(&SourceWithRecords {
                            source: &record.key_values(),
                            records: default_records,
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

struct SourceWithRecords<'a, I> {
    source: &'a dyn log::kv::Source,
    records: I,
}

impl<'a, I> log::kv::Source for SourceWithRecords<'a, I>
where
    I: Iterator<Item = LogRecordRef<'a>> + Clone,
{
    fn visit<'kvs>(
        &'kvs self,
        visitor: &mut dyn log::kv::VisitSource<'kvs>,
    ) -> Result<(), log::kv::Error> {
        for (key, value) in self.records.clone() {
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
