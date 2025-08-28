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

use std::borrow::Cow;

use stack::ContextRecords;

use self::stack::CONTEXT_STACK;
pub use self::{context::LogContext, future::FutureExt, value::ContextValue};

mod context;
pub mod future;
pub mod guard;
mod stack;
mod value;

type StaticCowStr = Cow<'static, str>;

/// A logger wrapper that enhances log records with contextual properties.
///
/// `ContextLogger` wraps an existing logging implementation and adds additional
/// context properties to log records. These context properties are taken from the
/// current context stack, which is managed by the [`LogContext`] type.
///
/// # Example
///
/// ```
/// use log::{info, LevelFilter};
/// use context_logger::{ContextLogger, LogContext};
///
/// // Create a logger.
/// let env_logger = env_logger::builder().build();
/// let max_level = env_logger.filter();
/// // Wrap it with ContextLogger to enable context propagation.
/// let context_logger = ContextLogger::new(env_logger);
/// // Initialize the resulting logger.
/// context_logger.init(max_level);
///
/// // Create a context with properties
/// let ctx = LogContext::new()
///     .record("request_id", "req-123")
///     .record("user_id", 42);
///
/// // Use the context while logging
/// let _guard = ctx.enter();
/// info!("Processing request"); // Will include request_id and user_id properties
/// ```
///
/// See [`LogContext`] for more information on how to create and manage context properties.
pub struct ContextLogger {
    default_records: ContextRecords,
    inner: Box<dyn log::Log>,
}

impl ContextLogger {
    /// Creates a new [`ContextLogger`] that wraps the given logging implementation.
    ///
    /// The inner logger will receive log records enhanced with context properties
    /// from the current context stack.
    pub fn new<L>(inner: L) -> Self
    where
        L: log::Log + 'static,
    {
        Self {
            default_records: ContextRecords::new(),
            inner: Box::new(inner),
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
    /// use context_logger::{ContextLogger, LogContext};
    ///
    /// // Create a logger with default records
    /// let logger = ContextLogger::new(env_logger::builder().build())
    ///     .default_record("service", "api")
    ///     .default_record("version", "1.0.0");
    /// // Initialize it
    /// logger.init(LevelFilter::Info);
    /// // Context records are added after default records
    /// let _guard = LogContext::new()
    ///     .record("request_id", "123")
    ///     .enter();
    ///
    /// info!("Processing request"); // Will include service="api", version="1.0.0", request_id="123"
    /// ```
    #[must_use]
    pub fn default_record(
        mut self,
        key: impl Into<StaticCowStr>,
        value: impl Into<ContextValue>,
    ) -> Self {
        self.default_records.push((key.into(), value.into()));
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
        let error = CONTEXT_STACK.try_with(|stack| {
            if let Some(top) = stack.top() {
                let extra_records = ExtraRecords {
                    source: &record.key_values(),
                    default_records: self.default_records.as_slice(),
                    context_records: top.as_slice(),
                };
                self.inner
                    .log(&record.to_builder().key_values(&extra_records).build());
            } else {
                let extra_records = ExtraRecords {
                    source: &record.key_values(),
                    default_records: self.default_records.as_slice(),
                    context_records: &[],
                };
                self.inner
                    .log(&record.to_builder().key_values(&extra_records).build());
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

struct ExtraRecords<'a, I> {
    source: &'a dyn log::kv::Source,
    default_records: I,
    context_records: I,
}

impl<'a, I> log::kv::Source for ExtraRecords<'a, I>
where
    I: IntoIterator<Item = &'a (StaticCowStr, ContextValue)> + Copy,
{
    fn visit<'kvs>(
        &'kvs self,
        visitor: &mut dyn log::kv::VisitSource<'kvs>,
    ) -> Result<(), log::kv::Error> {
        let all_records = self.default_records.into_iter().chain(self.context_records);
        for (key, value) in all_records {
            visitor.visit_pair(log::kv::Key::from_str(key), value.as_log_value())?;
        }
        self.source.visit(visitor)
    }
}
