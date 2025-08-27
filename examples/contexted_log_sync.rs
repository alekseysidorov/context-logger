use context_logger::{ContextLogger, ContextValue, LogContext};
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
        .with_default_record("instance", "contexted_log_sync")
        .try_init(level)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    try_init_logger()?;

    log::info!("Initialized context logger");

    // Create a new context with properties
    {
        let _guard = LogContext::new().record("user_id", "12345").enter();

        log::info!("Logging in");

        // Create a nested context with additional properties
        {
            let _nested_guard = LogContext::new()
                .record(
                    "action",
                    ContextValue::serde(Operation {
                        action: "login".to_string(),
                        name: "user".to_string(),
                    }),
                )
                .enter();
            log::info!("User logged in successfully");
        }

        log::info!("Login completed");
    }

    Ok(())
}
