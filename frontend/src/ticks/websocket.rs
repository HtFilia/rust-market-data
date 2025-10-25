use std::{rc::Rc, time::Duration};

use futures::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use gloo_timers::future::sleep;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use super::types::Tick;

#[derive(serde::Deserialize)]
struct TickBatchPayload {
    #[allow(dead_code)]
    version: u32,
    #[serde(default)]
    ticks: Vec<Tick>,
}

/// Errors that can surface when managing the websocket connection.
#[derive(Debug)]
pub enum TickStreamError {
    Open(String),
    Deserialize(String),
}

pub type TickCallback = Rc<dyn Fn(Vec<Tick>)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamStatus {
    Idle,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
    Failed,
}

pub type StatusCallback = Rc<dyn Fn(StreamStatus)>;

/// Connect to the tick stream with automatic reconnection and status updates.
pub fn connect_with_retry(url: String, on_tick: TickCallback, on_status: StatusCallback) {
    spawn_local(async move {
        let mut attempt: u32 = 0;
        let mut backoff_ms: u64 = 500;
        let mut ever_connected = false;

        loop {
            if attempt == 0 || !ever_connected {
                on_status(StreamStatus::Connecting);
            } else {
                on_status(StreamStatus::Reconnecting { attempt });
            }

            match WebSocket::open(&url) {
                Ok(ws) => {
                    attempt = 0;
                    backoff_ms = 500;

                    let (_, mut read) = ws.split();
                    let mut announced_connected = false;

                    while let Some(message) = read.next().await {
                        match message {
                            Ok(Message::Bytes(bytes)) => {
                                if let Err(err) = dispatch_message(&bytes, &on_tick) {
                                    log::warn!("dropping malformed tick: {err:?}");
                                } else if !announced_connected {
                                    announced_connected = true;
                                    ever_connected = true;
                                    on_status(StreamStatus::Connected);
                                }
                            }
                            Ok(Message::Text(text)) => {
                                if let Err(err) = dispatch_message(text.as_bytes(), &on_tick) {
                                    log::warn!("dropping malformed tick: {err:?}");
                                } else if !announced_connected {
                                    announced_connected = true;
                                    ever_connected = true;
                                    on_status(StreamStatus::Connected);
                                }
                            }
                            Err(err) => {
                                log::warn!("websocket read error: {err:?}");
                                break;
                            }
                        }
                    }

                    on_status(StreamStatus::Failed);
                }
                Err(err) => {
                    log::error!("websocket open error: {err:?}");
                    on_status(StreamStatus::Failed);
                }
            }

            attempt = attempt.saturating_add(1);
            sleep(Duration::from_millis(backoff_ms)).await;
            backoff_ms = (backoff_ms * 2).min(10_000);
        }
    });
}

fn dispatch_message(bytes: &[u8], on_tick: &TickCallback) -> Result<(), TickStreamError> {
    let payload: TickBatchPayload = serde_json::from_slice(bytes)
        .map_err(|err| TickStreamError::Deserialize(err.to_string()))?;

    if !payload.ticks.is_empty() {
        on_tick(payload.ticks);
    }
    Ok(())
}

impl From<TickStreamError> for JsValue {
    fn from(value: TickStreamError) -> Self {
        match value {
            TickStreamError::Open(err) => {
                JsValue::from_str(&format!("websocket open error: {err}"))
            }
            TickStreamError::Deserialize(err) => JsValue::from_str(&err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn dispatch_message_parses_tick_batches() {
        let captured: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let sink = captured.clone();
        let callback: TickCallback = Rc::new(move |ticks: Vec<Tick>| {
            sink.borrow_mut()
                .extend(ticks.into_iter().map(|tick| tick.symbol));
        });

        let payload = r#"{"version":1,"ticks":[{"symbol":"AAA","price":10.0,"timestamp_ms":1,"region":"north_america","sector":"technology"}]}"#;
        dispatch_message(payload.as_bytes(), &callback).expect("valid payload");

        let captured = captured.borrow();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0], "AAA");
    }
}
