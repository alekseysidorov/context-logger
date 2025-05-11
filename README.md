# context-logger

[![Crates.io](https://img.shields.io/crates/v/context-logger.svg)](https://crates.io/crates/context-logger)
[![Documentation](https://docs.rs/context-logger/badge.svg)](https://docs.rs/context-logger)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

<!-- ANCHOR: description -->

A lightweight, ergonomic library for adding structured context to your logs.

`context-logger` enhances the standard Rust [`log`] crate ecosystem by allowing
you to attach rich contextual information to your log messages without changing
your existing logging patterns.

<!-- ANCHOR_END: description -->

## Usage

### Basic Example

Add `context-logger` to your `Cargo.toml`:

```toml
[dependencies]
context-logger = "0.1.0"
log = { version = "0.4", features = ["kv_serde"] }
env_logger = "0.10"
```

Then, you can use it in your code:

<!-- ANCHOR: basic_example -->

```rust
use context_logger::{ContextLogger, LogContext};
use log::info;

fn main() {
    // Create a logger.
    let env_logger = env_logger::builder().build();
    let max_level = env_logger.filter();
    // Wrap it with ContextLogger to enable context propagation.
    let context_logger = ContextLogger::new(env_logger);
    // Initialize the resulting logger.
    context_logger.init(max_level);   

    // Create a context
    let ctx = LogContext::new()
        .record("request_id", "req-123")
        .record("user_id", 42);
    
    // Use the context
    let _guard = ctx.enter();
    
    // Log with context automatically attached
    info!("Processing request"); // Will include request_id=req-123 and user_id=42
}
```

<!-- ANCHOR_END: basic_example -->

### Async Context Propagation

<!-- ANCHOR: async_example -->

Context logger supports async functions and can propagate log context across
`.await` points.

```rust
use context_logger::{ContextLogger, LogContext, FutureExt};
use log::info;

async fn process_user_data(user_id: &str) {
    let context = LogContext::new().record("user_id", user_id);
    
    async {
        info!("Processing user data"); // Includes user_id
        
        // Context automatically propagates through .await points
        fetch_user_preferences().await;
        
        info!("User data processed"); // Still includes user_id
    }
    .in_log_context(context)
    .await;
}

async fn fetch_user_preferences() {
    // Add additional context for this specific operation
    LogContext::add_record("operation", "fetch_preferences");
    info!("Fetching preferences"); // Includes both user_id and operation
}
```

<!-- ANCHOR_END: async_example -->

## License

This project is licensed under the MIT License. See the [LICENSE] file for
details.

[`log`]: https://crates.io/crates/log
[LICENSE]: ./LICENSE
