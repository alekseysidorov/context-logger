use context_logger::log_scope;

#[log_scope(
    inherited_fields(request_id),
    local_fields(operation = "load", user:? = user, "http.method":% = method),
)]
async fn load(request_id: String, user: u32, method: String) {}

#[log_scope(local_fields(value:debug = value))]
fn sync(value: String) {}

#[log_scope]
fn empty() {}

#[log_scope(local_fields(), inherited_fields())]
fn empty_sections() {}

#[log_scope(
    local_fields(
        r#type,
        "http.method" = method.clone(),
        display_value:display = method.clone(),
        error_value:err = error,
    ),
)]
fn all_named_modes(r#type: u32, method: String, error: std::io::Error) {}

#[log_scope(local_fields(value = value))]
fn generic<T: Into<context_logger::LogValue>>(value: T) {}

fn main() {}
