use context_logger::log_scope;

#[log_scope(inherited_fields(value = 1), local_fields(value = 2))]
fn invalid() {}

fn main() {}
