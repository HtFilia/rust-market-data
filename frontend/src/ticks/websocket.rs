use std::rc::Rc;

use futures::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
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

/// Connect to the tick stream and invoke `on_tick` for every parsed payload batch.
pub fn spawn_tick_stream(url: &str, on_tick: TickCallback) -> Result<(), TickStreamError> {
    let ws = WebSocket::open(url).map_err(|err| TickStreamError::Open(err.to_string()))?;
    let (_, mut read) = ws.split();
    let on_tick = on_tick.clone();

    spawn_local(async move {
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Bytes(bytes)) => {
                    if let Err(err) = dispatch_message(&bytes, &on_tick) {
                        log::warn!("dropping malformed tick: {err:?}");
                    }
                }
                Ok(Message::Text(text)) => {
                    if let Err(err) = dispatch_message(text.as_bytes(), &on_tick) {
                        log::warn!("dropping malformed tick: {err:?}");
                    }
                }
                Err(err) => {
                    log::error!("websocket read error: {err:?}");
                    break;
                }
            }
        }
    });

    Ok(())
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
