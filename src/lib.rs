//! # Context Logger
//!
//! A lightweight, ergonomic library for adding structured context to your logs in both synchronous
//! and asynchronous Rust applications.
//!
//! `context-logger` enhances the standard Rust [`log`] ecosystem by allowing you to attach rich
//! contextual data to log messages without changing your existing logging patterns.
//!
//! ## Overview
//!
//! Modern applications often need rich, structured context in logs to provide insights into
//! runtime behavior. This library simplifies the process by:
//!
//! * Adding structured key-value pairs to logs without modifying existing log statements
//! * Propagating context through asynchronous execution boundaries
//! * Supporting nested contexts to build hierarchical relationships
//! * Preserving strong typing while allowing flexible value types
//!
//! ## Key Components
//!
//! * [`ContextLogger`] - A wrapper around any logger implementation that adds contextual properties
//! * [`LogContext`] - A container for creating and managing context properties
//! * [`ContextValue`] - A flexible container for various types of data
//! * [`FutureExt`] - Extension trait for propagating context through asynchronous code
//!
//! ## Basic Example
//!
//! ```
//! # fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use context_logger::{ContextLogger, LogContext};
//! use log::info;
//!
//! // Initialize the logger
//! let env_logger = env_logger::builder().build();
//! ContextLogger::new(env_logger).init(log::LevelFilter::Info);
//!
//! // Create a context with properties
//! let ctx = LogContext::new()
//!     .record("request_id", "req-123")
//!     .record("user_id", 42);
//!
//! // Enter the context
//! let _guard = ctx.enter();
//!
//! // Log with context automatically attached
//! info!("Processing request"); // Will include request_id and user_id
//! # Ok(())
//! # }
//! ```
//!
//! ## Async Context Propagation
//!
//! ```
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use context_logger::{ContextLogger, LogContext, FutureExt};
//! use log::info;
//!
//! // Create a context for this async operation
//! let context = LogContext::new()
//!     .record("operation_id", "op-456")
//!     .record("user_id", "user-123");
//!
//! // Apply the context to an async block
//! async {
//!     info!("Starting operation"); // Includes context properties
//!
//!     // Context automatically propagates through .await points
//!     some_async_function().await;
//!
//!     info!("Operation completed"); // Still includes context
//! }
//! .in_log_context(context)
//! .await;
//! # Ok(())
//! # }
//! # async fn some_async_function() {}
//! ```

use std::borrow::Cow;

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
/// This wrapper is compatible with any logger that implements the [`log::Log`] trait.
/// The context properties are automatically added to log records without modifying
/// existing logging code or patterns.
///
/// # Example
///
/// ```
/// use log::{info, LevelFilter};
/// use context_logger::{ContextLogger, LogContext};
///
/// // Create a logger with context support
/// let logger = ContextLogger::new(env_logger::Builder::new().build());
/// logger.init(LevelFilter::Info);
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
            inner: Box::new(inner),
        }
    }

    /// Initializes the global logger with the context logger.
    ///
    /// This should be called early in the execution of a Rust program. Any log events
    /// that occur before initialization will be ignored.
    ///
    /// # Panics
    ///
    /// Panics if a logger has already been set.
    ///
    /// # Example
    ///
    /// ```
    /// use context_logger::ContextLogger;
    /// use log::LevelFilter;
    ///
    /// let env_logger = env_logger::builder()
    ///     .filter_level(LevelFilter::Info)
    ///     .build();
    ///
    /// // Initialize the global logger
    /// ContextLogger::new(env_logger).init(LevelFilter::Info);
    /// ```
    pub fn init(self, max_level: log::LevelFilter) {
        self.try_init(max_level)
            .expect("ContextLogger::init should not be called after logger initialization");
    }

    /// Initializes the global logger with the context logger.
    ///
    /// This should be called early in the execution of a Rust program. Any log events
    /// that occur before initialization will be ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if a logger has already been set.
    ///
    /// # Example
    ///
    /// ```
    /// use context_logger::ContextLogger;
    /// use log::LevelFilter;
    ///
    /// let env_logger = env_logger::builder()
    ///     .filter_level(LevelFilter::Info)
    ///     .build();
    ///
    /// // Initialize the global logger, handling potential errors
    /// if let Err(err) = ContextLogger::new(env_logger).try_init(LevelFilter::Info) {
    ///     eprintln!("Failed to initialize logger: {}", err);
    /// }
    /// ```
    pub fn try_init(self, max_level: log::LevelFilter) -> Result<(), log::SetLoggerError> {
        log::set_max_level(max_level);
        log::set_boxed_logger(self.inner)
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
        let _ = CONTEXT_STACK.try_with(|stack| {
            if let Some(top) = stack.top() {
                let extra_properties = ExtraProperties {
                    source: &record.key_values(),
                    properties: &*top.properties,
                };
                let new_record = record.to_builder().key_values(&extra_properties).build();

                self.inner.log(&new_record);
            } else {
                self.inner.log(record);
            }
        });
    }

    fn flush(&self) {
        self.inner.flush();
    }
}

struct ExtraProperties<'a, I> {
    source: &'a dyn log::kv::Source,
    properties: I,
}

impl<'a, I> log::kv::Source for ExtraProperties<'a, I>
where
    I: IntoIterator<Item = &'a (StaticCowStr, ContextValue)> + Copy,
{
    fn visit<'kvs>(
        &'kvs self,
        visitor: &mut dyn log::kv::VisitSource<'kvs>,
    ) -> Result<(), log::kv::Error> {
        for (key, value) in self.properties {
            visitor.visit_pair(log::kv::Key::from_str(key), value.as_log_value())?;
        }
        self.source.visit(visitor)
    }
}
