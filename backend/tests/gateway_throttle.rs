use std::{
    collections::{HashMap, HashSet},
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use rust_market_data::{
    simulator::{self, SimulatorConfig},
    tick::Tick,
};
use serde::Deserialize;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn gateway_batches_symbols_once_per_interval() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9123);
    let config = SimulatorConfig {
        enable_socket: false,
        gateway_addr: addr,
        gateway_throttle: Duration::from_secs(1),
        tick_interval: Duration::from_millis(4),
        max_ticks: None,
        ..SimulatorConfig::default()
    };

    let simulator_task = tokio::spawn(async move {
        simulator::run_with_config(config)
            .await
            .expect("simulator run");
    });

    let mut attempts = 0usize;
    let (mut ws_stream, _) = loop {
        match tokio_tungstenite::connect_async(format!("ws://{addr}/ws")).await {
            Ok(conn) => break conn,
            Err(WsError::Io(err))
                if err.kind() == ErrorKind::ConnectionRefused && attempts < 20 =>
            {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            Err(err) => panic!("connect websocket: {err:?}"),
        }
    };

    let mut frames: Vec<(Instant, Vec<Tick>)> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(8);

    while Instant::now() < deadline {
        if let Some(message) = ws_stream.next().await {
            let message = message.expect("websocket message");
            if let Message::Text(payload) = message {
                let batch: TickBatchPayload =
                    serde_json::from_str(&payload).expect("tick payload batch");
                assert_eq!(
                    batch.version, 1,
                    "unexpected batch version {}",
                    batch.version
                );
                let ticks = batch.ticks;
                let unique: HashSet<_> = ticks.iter().map(|tick| tick.symbol.as_str()).collect();
                if unique.len() >= 400 {
                    frames.push((Instant::now(), ticks));
                }
                if frames.len() >= 2 {
                    break;
                }
            }
        }
    }

    assert!(
        frames.len() >= 2,
        "expected at least two aggregated frames from gateway"
    );

    for (_, ticks) in &frames {
        let mut seen: HashMap<&str, u64> = HashMap::new();
        for tick in ticks {
            *seen.entry(tick.symbol.as_str()).or_default() += 1;
        }
        assert!(
            seen.len() >= 400,
            "expected batch to contain hundreds of unique symbols, saw {}",
            seen.len()
        );
        assert!(
            seen.values().all(|count| *count == 1),
            "each symbol should appear once per batch"
        );
    }

    let interval = frames[1].0.duration_since(frames[0].0);
    assert!(
        interval >= Duration::from_millis(800),
        "expected roughly 1s between batches, observed {:?}",
        interval
    );

    let _ = ws_stream.close(None).await;
    simulator_task.abort();
    let _ = simulator_task.await;
}
#[derive(Deserialize)]
struct TickBatchPayload {
    version: u32,
    ticks: Vec<Tick>,
}
