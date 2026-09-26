use context_logger::{LogScope, log_scope};

fn value(context: &context_logger::LogContext, key: &str) -> Option<String> {
    context
        .local
        .iter()
        .chain(context.inherited.iter())
        .find(|(name, _)| name.as_ref() == key)
        .map(|(_, value)| value.to_string())
}

#[log_scope(inherited_fields(request_id), local_fields(operation = "sync"))]
fn sync_scope(request_id: String) -> context_logger::LogContext {
    LogScope::current_context()
}

#[log_scope(
    local_fields(value:% = value),
    inherited_fields(request_id = request_id),
)]
async fn async_scope(request_id: String, value: String) -> context_logger::LogContext {
    tokio::task::yield_now().await;
    LogScope::current_context()
}

#[log_scope(local_fields(a, b, c))]
fn shorthand_scope(a: u32, b: u32, c: u32) -> context_logger::LogContext {
    LogScope::current_context()
}

#[test]
fn sync_scope_uses_both_sections() {
    let context = sync_scope("req-1".to_owned());
    assert_eq!(value(&context, "request_id").as_deref(), Some("req-1"));
    assert_eq!(value(&context, "operation").as_deref(), Some("sync"));
}

#[test]
fn shorthand_fields_use_argument_names_as_keys() {
    let context = shorthand_scope(1, 2, 3);
    assert_eq!(value(&context, "a").as_deref(), Some("1"));
    assert_eq!(value(&context, "b").as_deref(), Some("2"));
    assert_eq!(value(&context, "c").as_deref(), Some("3"));
}

#[tokio::test]
async fn async_scope_survives_await() {
    let context = async_scope("req-2".to_owned(), "value".to_owned()).await;
    assert_eq!(value(&context, "request_id").as_deref(), Some("req-2"));
    assert_eq!(value(&context, "value").as_deref(), Some("value"));
}
