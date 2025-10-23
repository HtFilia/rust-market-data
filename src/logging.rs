use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize)]
struct LogEvent<'a> {
    level: &'a str,
    event: &'a str,
    message: &'a str,
    timestamp_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
}

fn emit(level: &str, event: &str, message: &str, metadata: Option<Value>) {
    let entry = LogEvent {
        level,
        event,
        message,
        timestamp_ms: current_timestamp_ms(),
        metadata,
    };

    match serde_json::to_string(&entry) {
        Ok(payload) => {
            if level == "error" {
                eprintln!("{payload}");
            } else {
                println!("{payload}");
            }
        }
        Err(err) => eprintln!(
            "{{\"level\":\"error\",\"event\":\"logging_failure\",\"message\":\"failed to serialise log\",\"error\":\"{err}\"}}"
        ),
    }
}

pub fn info(event: &str, message: &str, metadata: Value) {
    emit("info", event, message, Some(metadata));
}

pub fn warn(event: &str, message: &str, metadata: Value) {
    emit("warn", event, message, Some(metadata));
}

pub fn error(event: &str, message: &str, metadata: Value) {
    emit("error", event, message, Some(metadata));
}

pub fn info_simple(event: &str, message: &str) {
    emit("info", event, message, None);
}

pub fn warn_simple(event: &str, message: &str) {
    emit("warn", event, message, None);
}

pub fn error_simple(event: &str, message: &str) {
    emit("error", event, message, None);
}

fn current_timestamp_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis()
}

pub fn metadata_from_pairs(pairs: &[(&str, Value)]) -> Value {
    let mut obj = serde_json::Map::with_capacity(pairs.len());
    for (key, value) in pairs {
        obj.insert((*key).to_string(), value.clone());
    }
    Value::Object(obj)
}

pub fn metadata_object() -> Value {
    json!({})
}
