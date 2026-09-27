use context_logger::log_scope;

#[log_scope(other_fields(value = 42))]
fn invalid() {}

fn main() {}
