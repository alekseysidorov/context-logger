use context_logger::log_scope;

#[log_scope(
    inherited_fields(request_id),
    local_fields(operation = "load", user:? = user, "http.method":% = method),
)]
async fn load(request_id: String, user: u32, method: String) {}

#[log_scope(local_fields(value:debug = value))]
fn sync(value: String) {}

fn main() {}
