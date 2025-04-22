use context::CONTEXT_STACK;
use context_properties::StaticCowStr;

pub use self::{
    context::{FutureExt, LogContext, LogContextFuture, LogContextGuard},
    context_properties::ContextValue,
};

mod context;
mod context_properties;
mod context_stack;

pub struct ContextLogger {
    inner: Box<dyn log::Log>,
}

impl ContextLogger {
    pub fn new<L: log::Log + 'static>(inner: L) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

impl log::Log for ContextLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        let _ = CONTEXT_STACK.try_with(|stack| {
            if let Some(properties) = stack.current_properties() {
                let extra_properties = ExtraProperties {
                    source: &record.key_values(),
                    properties: &*properties,
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
