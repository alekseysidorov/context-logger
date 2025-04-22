use std::{collections::BTreeMap, time::Duration};

use context_logger::{ContextLogger, ContextValue, FutureExt, LogContext};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Operation {
    action: String,
    name: String,
}

fn write_json<F>(f: &mut F, record: &log::Record) -> std::io::Result<()>
where
    F: std::io::Write,
{
    #[derive(Default)]
    struct Visitor<'a> {
        kv: BTreeMap<log::kv::Key<'a>, log::kv::Value<'a>>,
    }

    impl<'a> log::kv::VisitSource<'a> for Visitor<'a> {
        fn visit_pair(
            &mut self,
            key: log::kv::Key<'a>,
            val: log::kv::Value<'a>,
        ) -> Result<(), log::kv::Error> {
            self.kv.insert(key, val);
            Ok(())
        }
    }

    let mut visitor = Visitor::default();
    record.key_values().visit(&mut visitor).unwrap();

    #[derive(Debug, Serialize)]
    struct Entry<'a> {
        level: log::Level,
        msg: String,
        #[serde(flatten)]
        kv: BTreeMap<log::kv::Key<'a>, log::kv::Value<'a>>,
    }

    let entry = Entry {
        level: record.level(),
        msg: record.args().to_string(),
        kv: visitor.kv,
    };

    writeln!(f, "{}", serde_json::to_string(&entry).unwrap())
}

fn try_init_logger() -> Result<(), Box<dyn std::error::Error>> {
    let env_logger = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format(write_json)
        .target(env_logger::Target::Stdout)
        .build();
    let level = env_logger.filter();

    let context_logger = ContextLogger::new(env_logger);
    log::set_boxed_logger(Box::new(context_logger))?;
    log::set_max_level(level);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    try_init_logger()?;

    log::info!("Initialized context logger");

    // Create a new context with properties.
    let log_context = LogContext::new().with_property("user_id", "12345");
    let first_future = async move {
        log::info!("Logging in");
        // Create a nested context with additional properties
        let log_context = LogContext::new().with_property(
            "action",
            ContextValue::serde(Operation {
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
        .with_property("name", "Alice")
        .with_property("age", "25")
        .with_property("email", "alice@example.com");
    let second_future = async move {
        tokio::task::yield_now().await;

        log::info!("Another future pending");
        tokio::time::sleep(Duration::from_millis(100)).await;
        log::info!("Future completed");
    }
    .in_log_context(log_context);

    let log_context = LogContext::new()
        .with_property("name", "Bob")
        .with_property("age", "30")
        .with_property("email", "bob@example.com");
    let third_future = tokio::spawn(
        async move {
            tokio::task::yield_now().await;

            log::info!("Third future pending");
            tokio::time::sleep(Duration::from_millis(100)).await;

            LogContext::add_property(
                "operation",
                ContextValue::serde(Operation {
                    action: "logout".to_owned(),
                    name: "Bob".to_owned(),
                }),
            );
            log::info!("Third future completed");
        }
        .in_log_context(log_context),
    );

    let (_, _, res) = tokio::join!(first_future, second_future, third_future);
    res?;

    let context = LogContext::new()
        .with_property("name", "Charlie")
        .with_property("age", "35")
        .with_property("email", "charlie@example.com");

    let _guard = context.enter();

    log::info!("Last call completed");

    Ok(())
}
