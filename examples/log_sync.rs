use context_logger::{ContextLogger, LogContext, LogScope, LogValue};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Operation {
    action: String,
    name: String,
}

fn try_init_logger() -> Result<(), Box<dyn std::error::Error>> {
    let env_logger = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .build();
    let level = env_logger.filter();

    ContextLogger::new(env_logger)
        .default_record("instance", "contexted_log_sync")
        .try_init(level)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    try_init_logger()?;

    log::info!("Initialized context logger");

    // Create a new context with properties
    {
        let _guard = LogScope::enter(
            LogContext::new()
                .inherited_record("example", "sync")
                .local_record("user_id", "12345"),
        );

        log::info!("Logging in");

        // Create a nested context with additional properties
        {
            let context = LogContext::new().local_record(
                "action",
                LogValue::serde(Operation {
                    action: "login".to_string(),
                    name: "user".to_string(),
                }),
            );
            let _nested_guard = LogScope::enter(context);
            log::info!("User logged in successfully");
        }

        log::info!("Login completed");
    }

    Ok(())
}
