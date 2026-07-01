# context-logger

[![Crates.io](https://img.shields.io/crates/v/context-logger.svg)](https://crates.io/crates/context-logger)
[![Documentation](https://docs.rs/context-logger/badge.svg)](https://docs.rs/context-logger)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

<!-- ANCHOR: description -->

A small structured context propagation layer for the standard Rust [`log`]
ecosystem.

`context-logger` keeps your existing `log::info!`, `log::warn!`, etc. calls
unchanged and adds scoped structured context on top — inherited request-level
fields, local operation fields, computed defaults, and async-safe propagation.

<!-- ANCHOR_END: description -->

## Why context-logger?

The `log` crate is the de facto logging facade in Rust: small, stable, and
widely adopted. Many projects rely on it and do not want to migrate their
instrumentation model.

Structured key-value logging — attaching `request_id`, `user_id`, `timestamp` —
requires passing those values through every function call stack. With `tracing`
this is automatic via spans, but tracing brings a rich (and sometimes heavy)
instrumentation framework that some projects do not need.

`context-logger` fills the gap between raw `log` calls and full tracing. It
wraps any `log::Log` implementation and propagates scoped structured context
through function boundaries and across `.await` points, without requiring
callers to accept or forward additional parameters.

### Is this a replacement for tracing?

No.

Use `tracing` if you need spans, subscribers, layers, callsites, and a full
instrumentation framework.

Use `context-logger` if your project already uses `log` and you only need scoped
structured context propagation without migrating to a new logging crate.

## Usage

### Basic Example

Add `context-logger` to your `Cargo.toml`:

```toml
[dependencies]
context-logger = "0.2"
log = { version = "0.4", features = ["kv_serde"] }
env_logger = { version = "0.11", features = ["kv"] }
```

Then, you can use it in your code:

<!-- ANCHOR: basic_example -->

```rust
use context_logger::{ContextLogger, LogContext, LogContextExt as _};
use log::info;

fn main() {
    // Create an underlying logger instance.
    let env_logger = env_logger::builder()
         .filter_level(log::LevelFilter::Info)
         .build();
    let filter = env_logger.filter();
     // Wrap it with ContextLogger to enable context propagation.
    let logger = ContextLogger::new(env_logger)
         // Add static default records (static fields).
         .with_default_record("service", "api")
         // Add computed default records (per-event fields).
         .with_default_record_fn("timestamp", |_record| {
            chrono::Utc::now().to_rfc3339()
         })
         .with_default_record_fn("level", |record| record.level().to_string());
     // Initialize the resulting logger.
    logger.init(filter);

    // Create a context with properties.
    let context = LogContext::new()
        // Record that will be inherited by child contexts.
        .with_inherited_record("request_id", "req-123")
        // Local record that will only be present in this context.
        .with_local_record("user_id", 42);

    // Use the context.
    context.in_scope(|| {
        // Log with context automatically attached:
        // service=api version=1.0.0 request_id=req-123 user_id=42 timestamp="..."
        info!("Processing request");
    })
}
```

<!-- ANCHOR_END: basic_example -->

### Async Context Propagation

<!-- ANCHOR: async_example -->

Context logger supports async functions and can propagate log context across
`.await` points.

```rust
use context_logger::{ContextLogger, LogContext, FutureExt, LogScope};
use log::info;

async fn process_user_data(user_id: &str) {
    let context = LogContext::new()
        .with_local_record("user_id", user_id);

    async {
        info!("Processing user data"); // Includes user_id

        // Context automatically propagates through .await points.
        fetch_user_preferences().await;

        info!("User data processed"); // Still includes user_id
    }
    .in_log_context(context)
    .await;
}

async fn fetch_user_preferences() {
    // Add additional record for this operation.
    LogScope::add_record("operation", "fetch_preferences");
    info!("Fetching preferences"); // Includes both user_id and operation
}

async fn spawn_background_job(user_id: &str) {
    let context = LogContext::new()
        .with_local_record("user_id", user_id);

    async {
        // The scope stack is thread-local: capture the active context
        // before crossing the task boundary with tokio::spawn.
        let context = LogScope::current_context();
        tokio::spawn(
            async move {
                info!("Running background job"); // Includes user_id
            }
            .in_log_context(context),
        )
        .await
        .unwrap();
    }
    .in_log_context(context)
    .await;
}
```

<!-- ANCHOR_END: async_example -->

## License

This project is licensed under the MIT License. See the [LICENSE] file for
details.

[`log`]: https://crates.io/crates/log
[LICENSE]: ./LICENSE
