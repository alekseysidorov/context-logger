use std::time::Duration;

use context_logger::{ContextLogger, FutureExt, LogContext, LogScope, LogValue};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Operation {
    action: String,
    name: String,
}

fn try_init_logger() -> Result<(), Box<dyn std::error::Error>> {
    let level = log::LevelFilter::Info;

    let logger = structured_logger::Builder::with_level(level.as_str())
        .with_target_writer("*", structured_logger::json::new_writer(std::io::stdout()))
        .build();
    ContextLogger::new(logger)
        .default_record("instance", "contexted_log_async")
        .try_init(level)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    try_init_logger()?;

    log::info!("Initialized context logger");

    // Create a new context with properties.
    let log_context = LogContext::new()
        .inherited_record("thread_name", "first_future")
        .local_record("user_id", "12345");
    let first_future = async move {
        log::info!("Logging in");
        // Create a nested context with additional properties
        let log_context = LogContext::new().local_record(
            "action",
            LogValue::serde(Operation {
                action: "login".to_string(),
                name: "user".to_string(),
            }),
        );
        async move {
            log::info!("User logged in successfully");
            tokio::task::yield_now().await;
        }
        .in_log_context(log_context)
        .await;

        tokio::time::sleep(Duration::from_millis(100)).await;
        log::info!("Login completed");
    }
    .in_log_context(log_context);

    let log_context = LogContext::new()
        .inherited_record("thread_name", "second_future")
        .local_record("name", "Alice")
        .local_record("age", 25)
        .local_record("married", true)
        .local_record("email", "alice@example.com");
    let second_future = async move {
        tokio::task::yield_now().await;

        log::info!("Another future pending");
        tokio::time::sleep(Duration::from_millis(100)).await;
        log::info!("Future completed");
    }
    .in_log_context(log_context);

    let log_context = LogContext::new()
        .inherited_record("thread_name", "third_future")
        .local_record("name", "Bob")
        .local_record("age", 30)
        .local_record("email", "bob@example.com");
    let third_future = tokio::spawn(
        async move {
            tokio::task::yield_now().await;

            log::info!("Third future pending");
            tokio::time::sleep(Duration::from_millis(100)).await;

            LogScope::add_record(
                "operation",
                LogValue::serde(Operation {
                    action: "logout".to_owned(),
                    name: "Bob".to_owned(),
                }),
            );
            log::info!("Third future completed");
        }
        .in_log_context(log_context),
    );

    let ((), (), res) = tokio::join!(first_future, second_future, third_future);
    res?;

    let context = LogContext::new()
        .local_record("name", "Charlie")
        .local_record("age", 35)
        .local_record("email", "charlie@example.com");

    let _guard = LogScope::enter(context);

    log::info!("Last call completed");

    Ok(())
}
