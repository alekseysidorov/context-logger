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

## Why `context-logger`?

The standard `log` crate is a small and widely used logging facade, but it does
not provide scoped context propagation by itself. This becomes noisy when every
log line should carry fields like `request_id`, `user_id`, `tenant_id`, or
`operation`.

`context-logger` fills this gap by adding a small runtime context layer on top
of ordinary `log` calls:

- **Works with existing `log` calls** — keep using `log::info!`, `log::warn!`,
  and your existing logger backend.
- **Fully dynamic context** — create, extend, merge, and pass context as
  ordinary runtime data. Add fields at any point in your code without declaring
  spans.
- **Scoped structured fields** — attach request-level fields, operation-level
  fields, and default fields to every log record, with explicit control over
  what is inherited by child scopes.
- **Async propagation** — carry context through `.await` with `FutureExt`.

### Is this a replacement for `tracing`?

> No. `context-logger` is focused on logs, not traces.

`tracing` puts logs and spans into one instrumentation model.

`context-logger` keeps logs as regular `log` records and adds scoped structured
context to them. Context remains ordinary runtime data, so existing
`log::info!`, `log::warn!`, etc. calls stay unchanged.

> What about traces?

Use a dedicated tracing library such as [`fastrace`].

This keeps the two concerns separate:

- `context-logger` enriches ordinary `log` records with scoped structured
  context;
- `fastrace` records timeline spans and exports traces through OpenTelemetry.

Together they cover structured logs and distributed traces without requiring all
logging to go through the `tracing` instrumentation model.

## Usage

### Basic Example

Add `context-logger` to your `Cargo.toml`:

```toml
[dependencies]
chrono = "0.4"
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
        .with_default_record_fn("timestamp", |_log_record| {
            chrono::Utc::now().to_rfc3339()
        })
        .with_default_record_fn("level", |record| record.level().to_string());
    // Initialize the resulting logger.
    logger.init(filter);

    // Create a context with properties.
    let context = LogContext::new()
        // Record that will be inherited by child contexts.
        .with_inherited_record("request_id", "req-123")
        .with_inherited_record("user_id", 42);

    // Use the context.
    context.in_scope(|| {
        // Log with context automatically attached:
        // service=api timestamp=... level=INFO request_id=req-123 user_id=42
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
        .with_inherited_record("user_id", user_id);

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
        .with_inherited_record("user_id", user_id);

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
[`fastrace`]: https://docs.rs/fastrace
