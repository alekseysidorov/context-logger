use std::fmt;

use context_logger::{ContextLogger, LogScope, log_scope};
use serde::Serialize;
use serde_json::{Value, json};

mod common;

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

#[log_scope(inherited_fields(value = "inherited"), local_fields(value = "local"))]
fn same_key_scope() {
    log::info!("same key");
}

#[log_scope(local_fields(r#type))]
fn raw_identifier_scope(r#type: u32) {
    log::info!("raw identifier");
}

#[log_scope(
    inherited_fields(request_id),
    local_fields(operation = "sync", user = user),
)]
fn logged_sync(request_id: String, user: u32) {
    log::info!("sync log");
}

#[log_scope(inherited_fields(request_id), local_fields(operation = "async"))]
async fn logged_async(request_id: String) {
    log::info!("before await");
    tokio::task::yield_now().await;
    log::info!("after await");
}

#[derive(Debug, Serialize)]
struct SerdeValue {
    answer: u32,
}

#[derive(Debug)]
struct ErrorValue;

impl fmt::Display for ErrorValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("error value")
    }
}

impl std::error::Error for ErrorValue {}

#[log_scope(local_fields(
    short_debug: ? = debug_value,
    named_debug:debug = named_debug,
    short_display: % = display_value,
    named_display:display = named_display,
    error:err = error,
    serde:serde = serde,
))]
fn logged_modes(
    debug_value: u32,
    named_debug: u32,
    display_value: String,
    named_display: String,
    error: ErrorValue,
    serde: SerdeValue,
) {
    log::info!("mode log");
}

#[test]
// Verify that both generated section kinds contribute to a synchronous scope.
fn sync_scope_uses_both_sections() {
    let context = sync_scope("req-1".to_owned());
    assert_eq!(value(&context, "request_id").as_deref(), Some("req-1"));
    assert_eq!(value(&context, "operation").as_deref(), Some("sync"));
}

#[test]
// Verify that identifier shorthand uses the parameter name as the field key.
fn shorthand_fields_use_argument_names_as_keys() {
    let context = shorthand_scope(1, 2, 3);
    assert_eq!(value(&context, "a").as_deref(), Some("1"));
    assert_eq!(value(&context, "b").as_deref(), Some("2"));
    assert_eq!(value(&context, "c").as_deref(), Some("3"));
}

#[tokio::test]
// Verify async scope propagation across await points and cleanup after return.
async fn async_scope_survives_await() {
    let context = async_scope("req-2".to_owned(), "value".to_owned()).await;
    assert_eq!(value(&context, "request_id").as_deref(), Some("req-2"));
    assert_eq!(value(&context, "value").as_deref(), Some("value"));
}

#[tokio::test]
// Verify generated scopes enrich real log records and support every capture mode.
async fn derived_scope_enriches_sync_and_async_records() {
    let (logger, records) = common::init_channel_logger();
    ContextLogger::new(logger).init(log::LevelFilter::Trace);

    logged_sync("req-sync".to_owned(), 42);
    let sync_record = records.recv().unwrap();
    assert_eq!(sync_record.message, "sync log");
    assert_eq!(
        Value::from(sync_record.fields),
        json!({
            "request_id": "req-sync",
            "operation": "sync",
            "user": 42,
        })
    );

    logged_async("req-async".to_owned()).await;
    for message in ["before await", "after await"] {
        let record = records.recv().unwrap();
        assert_eq!(record.message, message);
        assert_eq!(
            Value::from(record.fields),
            json!({
                "request_id": "req-async",
                "operation": "async",
            })
        );
    }

    log::info!("outside log");
    let outside_record = records.recv().unwrap();
    assert_eq!(outside_record.message, "outside log");
    assert_eq!(Value::from(outside_record.fields), json!({}));

    logged_modes(
        1,
        2,
        "display".to_owned(),
        "named display".to_owned(),
        ErrorValue,
        SerdeValue { answer: 42 },
    );
    let modes_record = records.recv().unwrap();
    assert_eq!(modes_record.message, "mode log");
    assert_eq!(
        Value::from(modes_record.fields),
        json!({
            "short_debug": "1",
            "named_debug": "2",
            "short_display": "display",
            "named_display": "named display",
            "error": "error value",
            "serde": { "answer": 42 },
        })
    );

    same_key_scope();
    let shadowed_record = records.recv().unwrap();
    assert_eq!(shadowed_record.message, "same key");
    assert_eq!(
        Value::from(shadowed_record.fields),
        json!({"value": "local"})
    );

    raw_identifier_scope(42);
    let raw_identifier_record = records.recv().unwrap();
    assert_eq!(raw_identifier_record.message, "raw identifier");
    assert_eq!(
        Value::from(raw_identifier_record.fields),
        json!({"r#type": 42})
    );
}
