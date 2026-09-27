use context_logger::log_scope;

#[log_scope(local_fields(value = 1, value = 2))]
fn invalid() {}

fn main() {}
