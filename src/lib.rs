//! # context-logger
//!
//! A library for adding structured contextual data to log entries using the standard log crate.
//!
//! `context-logger` enhances Rust's logging infrastructure by allowing you to associate
//! key-value pairs with log entries, regardless of where in your application's code
//! those log entries are emitted. This contextual data propagates across threads and
//! asynchronous tasks, providing consistent, structured logging throughout your
//! application's lifecycle.
//!
//! ## Key Features
//!
//! - **Contextual Data**: Attach key-value pairs to log statements without modifying them
//! - **Async Support**: Propagate context across async tasks and futures with `.in_log_context()`
//! - **Flexible Values**: Support for primitives, strings, errors, and `serde::Serialize` types
//! - **Nested Contexts**: Create hierarchical contexts for more granular logging
//!
//! ## Usage Examples
//!
//! ### Synchronous Context
//!
//! ```rust
//! use log::{info, LevelFilter};
//! use context_logger::{ContextLogger, LogContext};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize the logger
//! let logger = ContextLogger::new(env_logger::Builder::new().build());
//! logger.init(LevelFilter::Info);
//!
//! // Create a context and enter it
//! {
//!     let _guard = LogContext::new()
//!         .record("request_id", "req-123")
//!         .record("user_id", 42)
//!         .enter();
//!
//!     // Log statements in this scope will include the contextual data
//!     info!("Processing request");
//! } // Context is automatically removed when guard is dropped
//! # Ok(())
//! # }
//! ```
//!
//! ### Asynchronous Context
//!
//! ```rust
//! use log::info;
//! use context_logger::{LogContext, FutureExt};
//!
//! async fn process_data() {
//!     // Create a context for this async task
//!     let context = LogContext::new()
//!         .record("operation", "data_processing")
//!         .record("batch_id", "batch-456");
//!
//!     async {
//!         info!("Starting data processing"); // Has access to the context
//!         
//!         // Process data...
//!         
//!         info!("Completed data processing"); // Still has context
//!     }
//!     .in_log_context(context)
//!     .await;
//! }
//! ```
//!
//! ### Adding Context Dynamically
//!
//! ```rust
//! use log::info;
//! use context_logger::{LogContext, ContextValue};
//!
//! # fn example() {
//! # let _guard = LogContext::new().enter();
//! // Within a context scope...
//! info!("Starting operation");
//!
//! // Add data to the current context
//! LogContext::add_record("duration_ms", 42);
//! LogContext::add_record("status", "completed");
//!
//! info!("Operation complete"); // Will include added records
//! # }
//! ```
//!
//! ## Core Components
//!
//! - [`ContextLogger`]: Wrapper for a log implementation that adds context to log records
//! - [`LogContext`]: Builder for creating context with key-value pairs
//! - [`ContextValue`]: Container for different value types in the context
//! - [`FutureExt`]: Extension trait for futures to propagate context
//! - [`guard::LogContextGuard`]: RAII guard for managing context lifetimes

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
/// # Example
///
/// ```
/// use log::{info, LevelFilter};
/// use context_logger::{ContextLogger, LogContext};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
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
/// # Ok(())
/// # }
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

    /// Initialized the global logger with the context logger.
    ///
    /// This should be called early in the execution of a Rust program. Any log events that occur before initialization will be ignored.
    ///
    /// # Panics
    ///
    /// Returns an error if a logger has already been set.
    pub fn init(self, max_level: log::LevelFilter) {
        self.try_init(max_level)
            .expect("ContextLogger::init should not be called after logger initialization");
    }

    /// Initialized the global logger with the context logger.
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
