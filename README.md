# context-logger

[![Crates.io](https://img.shields.io/crates/v/context-logger.svg)](https://crates.io/crates/context-logger)
[![Documentation](https://docs.rs/context-logger/badge.svg)](https://docs.rs/context-logger)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A lightweight, ergonomic library for adding structured context to your logs in
both synchronous and asynchronous Rust applications.

`context-logger` enhances the standard Rust [log](https://crates.io/crates/log)
ecosystem by allowing you to attach rich contextual data to log messages without
changing your existing logging patterns.

## Features

- **Context Propagation**: Add structured context to log messages that
  automatically propagate through your application
- **Async Support**: First-class support for asynchronous code with context
  propagation through futures
- **Type-Safe**: Strongly typed context values with support for primitives,
  strings, and complex types
- **Nested Contexts**: Create hierarchical contexts that build on each other
- **Thread-Safe**: Safely use contexts across thread boundaries
- **Flexible Values**: Support for any type implementing `Debug`, `Display`,
  `Error`, or `serde::Serialize`
- **Zero-Cost Abstraction**: Only pay for what you use, with minimal runtime
  overhead
- **Minimal Dependencies**: Small dependency footprint

## Installation

Add `context-logger` to your `Cargo.toml`:

```toml
[dependencies]
context-logger = "0.1.0"
log = { version = "0.4", features = ["kv_serde"] }
```

## Quick Start

```rust
use context_logger::{ContextLogger, LogContext};
use log::info;

fn main() {
    // Initialize the logger
    let env_logger = env_logger::builder().build();
    ContextLogger::new(env_logger).init(log::LevelFilter::Info);

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

## Usage Examples

### Basic Synchronous Usage

```rust
use context_logger::{ContextLogger, LogContext};
use log::info;

fn process_request(user_id: &str) {
    // Create and enter a context
    let _guard = LogContext::new().record("user_id", user_id).enter();
    
    info!("Starting request processing"); // Includes user_id
    
    // Nested context
    {
        let _nested = LogContext::new().record("action", "validate").enter();
        info!("Validating user"); // Includes both user_id and action
    }
    
    info!("Request completed"); // Back to just user_id
}
```

### Async Context Propagation

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

### With Serializable Objects

```rust
use context_logger::{ContextLogger, ContextValue, LogContext};
use log::info;
use serde::Serialize;

#[derive(Serialize)]
struct User {
    name: String,
    role: String,
    id: u64,
}

fn log_with_complex_context() {
    let user = User {
        name: "Alice".to_string(),
        role: "Admin".to_string(),
        id: 42,
    };

    let _guard = LogContext::new()
        .record("user", ContextValue::serde(user))
        .enter();

    info!("User action performed"); // Will include serialized user object
}
```

## Integrating with Different Loggers

`context-logger` works with any logger that implements the `log::Log` trait:

```rust
// With env_logger
let env_logger = env_logger::builder().build();
ContextLogger::new(env_logger).init(log::LevelFilter::Info);

// With log4rs
let log4rs_logger = log4rs::init_config(config).unwrap();
ContextLogger::new(log4rs_logger).init(log::LevelFilter::Info);

// With simplelog
let file_logger = simplelog::WriteLogger::new(
    simplelog::LevelFilter::Info,
    simplelog::Config::default(),
    std::fs::File::create("app.log").unwrap()
);
ContextLogger::new(file_logger).init(log::LevelFilter::Info);
```

## Advanced Usage

### Custom Value Types

```rust
use context_logger::{ContextValue, LogContext};
use std::time::{Duration, Instant};

struct RequestTimer {
    start: Instant,
}

impl RequestTimer {
    fn new() -> Self {
        Self { start: Instant::now() }
    }
    
    fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl std::fmt::Display for RequestTimer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ms", self.elapsed().as_millis())
    }
}

fn timed_operation() {
    let timer = RequestTimer::new();
    
    // Use the Display impl
    let _guard = LogContext::new()
        .record("timer", ContextValue::display(timer))
        .enter();
        
    // Do some work...
    
    log::info!("Operation completed"); // Will include elapsed time
}
```

### Thread Boundaries

When working with threads:

```rust
use context_logger::{LogContext, ContextValue};
use log::info;
use std::thread;

fn main() {
    // Context in the main thread
    let ctx = LogContext::new().record("main_thread_id", 1);
    let _guard = ctx.enter();
    info!("In main thread"); // With main context
    
    // Spawn a new thread with its own context
    thread::spawn(|| {
        let thread_ctx = LogContext::new().record("worker_thread_id", 2);
        let _guard = thread_ctx.enter();
        
        info!("In worker thread"); // With worker context
    }).join().unwrap();
    
    info!("Back in main thread"); // Still with main context
}
```

## Performance Considerations

`context-logger` is designed to be lightweight and efficient. Context
propagation in async code has minimal overhead due to smart handling of future
polling. The context stack is thread-local, avoiding synchronization costs.

## License

This project is licensed under the [MIT License](LICENSE).
