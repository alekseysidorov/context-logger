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

    let context_logger = ContextLogger::new(env_logger);
    log::set_boxed_logger(Box::new(context_logger))?;
    log::set_max_level(level);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    try_init_logger()?;

    log::info!("Initialized context logger");

    // Create a new context with properties
    {
        let _guard = LogContext::new().with_property("user_id", "12345").enter();

        log::info!("Logging in");

        // Create a nested context with additional properties
        {
            let _nested_guard = LogContext::new()
                .with_property(
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
