use std::time::Duration;

use context_logger::{ContextLogger, FutureExt, LogContext, LogScope};

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
        .local_record("user_id", "12345")
        .inherited_record("global_record", "example");

    count_with_tokio_spawn(7).in_log_context(log_context).await;

    log::info!("Finished counting");

    Ok(())
}

async fn count_with_tokio_spawn(counter: u64) {
    log::info!("Invoked a function with detached work");

    // The scope stack is thread-local, so the active context is not visible
    // inside `tokio::spawn` by default. Capture it here and pass it explicitly.
    let context = LogScope::current_context();
    let handle = tokio::spawn(
        async move {
            for i in 0..counter {
                log::info!(i; "Counting...");
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
        .in_log_context(context),
    );

    handle.await.unwrap();
}
