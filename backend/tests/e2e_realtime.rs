use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use futures_util::StreamExt;
use rust_market_data::{
    simulator::{self, SimulatorConfig},
    tick::Tick,
};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

#[derive(Deserialize)]
struct TickBatchPayload {
    version: u32,
    ticks: Vec<Tick>,
}

async fn start_simulator() -> JoinHandle<()> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9124);
    let config = SimulatorConfig {
        enable_socket: false,
        gateway_addr: addr,
        gateway_throttle: Duration::from_millis(500),
        tick_interval: Duration::from_millis(4),
        max_ticks: None,
        ..SimulatorConfig::default()
    };

    tokio::spawn(async move {
        let _ = simulator::run_with_config(config).await;
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_stream_emits_batches() {
    let handle = start_simulator().await;

    let connect_addr = "ws://127.0.0.1:9124/ws";
    let (mut ws, _) = loop {
        match tokio_tungstenite::connect_async(connect_addr).await {
            Ok(conn) => break conn,
            Err(err) => {
                if let tokio_tungstenite::tungstenite::Error::Io(io) = &err {
                    if matches!(io.kind(), std::io::ErrorKind::ConnectionRefused) {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                }
                panic!("failed to connect to gateway: {err}");
            }
        }
    };

    let mut total_batches = 0usize;
    let mut total_ticks = 0usize;

    while total_batches < 3 {
        let maybe_message = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("websocket message timeout");

        match maybe_message {
            Some(Ok(Message::Text(payload))) => {
                let batch: TickBatchPayload =
                    serde_json::from_str(&payload).expect("valid payload");
                assert_eq!(batch.version, 1, "unexpected batch version");
                assert!(!batch.ticks.is_empty(), "empty batch received");
                total_batches += 1;
                total_ticks += batch.ticks.len();
            }
            Some(Ok(_)) => continue,
            Some(Err(err)) => panic!("websocket error: {err}"),
            None => break,
        }
    }

    assert!(total_batches > 0, "expected at least one batch");
    assert!(total_ticks > 0, "expected to receive ticks");

    let _ = ws.close(None).await;
    handle.abort();
}
