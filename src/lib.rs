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
