use context_logger::log_scope;

#[log_scope(local_fields(value:unknown = value))]
fn invalid(value: u32) {}

fn main() {}
