# context-logger

[![Crates.io](https://img.shields.io/crates/v/context-logger.svg)](https://crates.io/crates/context-logger)
[![Documentation](https://docs.rs/context-logger/badge.svg)](https://docs.rs/context-logger)

A library for enhancing Rust logs with structured contextual data that
propagates across threads and async tasks.

## Overview

`context-logger` seamlessly integrates with the standard `log` crate to provide
structured, contextual logging. It allows you to:

- Attach key-value pairs to log entries without modifying your logging calls
- Create nested contexts for hierarchical logging
- Propagate context across async tasks and threads
- Support for various data types including primitives, strings, complex objects,
  and serializable structures

## Features

- **Contextual Logging**: Add structured data to all logs within a scope
- **Async Support**: Automatically propagate context between async tasks using
  the `.in_log_context()` extension method
- **Nested Contexts**: Create nested contexts with different or additional
  properties
- **Serialization Support**: Store `serde::Serialize` objects in context
- **Transparent Integration**: Works with existing log crate formatters

## Quick Start

```rust
use context_logger::{ContextLogger, LogContext};
use log::info;

// Setup logger
let logger = ContextLogger::new(env_logger::builder().build());
logger.init(log::LevelFilter::Info);

// Create a context with structured data
{
    let _guard = LogContext::new()
        .record("request_id", "abc-123")
        .record("user_id", 42)
        .enter();

    // All log calls within this scope will include the context data
    info!("Processing request"); 
}
```

## Async Example

```rust
use context_logger::{FutureExt, LogContext};
use log::info;

async fn process_request() {
    let context = LogContext::new()
        .record("request_id", "abc-123")
        .record("user_id", 42);

    async {
        // All logs in this future will contain the context data
        info!("Starting processing");
        
        // Do some work...
        
        info!("Finished processing");
    }
    .in_log_context(context)
    .await;
}
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
context-logger = "0.1.0"
log = { version = "0.4", features = ["kv_serde"] }
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file
for details.
