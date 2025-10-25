use std::collections::hash_map::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch};
use tokio::time::{interval, MissedTickBehavior};

use crate::{logging, tick::Tick};

use super::ShutdownSignal;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulator_snapshot_sorts_symbols() {
        let mut accumulator = BatchAccumulator::default();
        accumulator.ingest(Tick {
            symbol: "B".into(),
            price: 1.0,
            timestamp_ms: 1,
            region: crate::model::Region::Europe,
            sector: crate::model::Sector::Technology,
        });
        accumulator.ingest(Tick {
            symbol: "A".into(),
            price: 1.0,
            timestamp_ms: 2,
            region: crate::model::Region::Europe,
            sector: crate::model::Sector::Technology,
        });

        let snapshot = accumulator.snapshot();
        let symbols: Vec<_> = snapshot.iter().map(|tick| tick.symbol.as_str()).collect();
        assert_eq!(symbols, vec!["A", "B"]);
    }
}

pub(super) async fn run_gateway(
    addr: SocketAddr,
    throttle: Duration,
    source_sender: broadcast::Sender<Tick>,
    aggregator_shutdown: watch::Receiver<ShutdownSignal>,
    server_shutdown: watch::Receiver<ShutdownSignal>,
) -> Result<()> {
    let (gateway_sender, _) = broadcast::channel::<Vec<Tick>>(64);

    tokio::try_join!(
        run_gateway_aggregator(
            throttle,
            source_sender.subscribe(),
            gateway_sender.clone(),
            aggregator_shutdown
        ),
        run_gateway_server(addr, gateway_sender, server_shutdown),
    )?;

    Ok(())
}

async fn run_gateway_aggregator(
    throttle: Duration,
    mut source: broadcast::Receiver<Tick>,
    gateway_sender: broadcast::Sender<Vec<Tick>>,
    mut shutdown: watch::Receiver<ShutdownSignal>,
) -> Result<()> {
    logging::info_simple("gateway.aggregator.start", "Gateway aggregator started");

    let mut accumulator = BatchAccumulator::default();
    let mut ticker = interval(throttle);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker.reset();

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if !accumulator.is_empty() {
                    let snapshot = accumulator.snapshot();
                    if !snapshot.is_empty() {
                        let _ = gateway_sender.send(snapshot);
                    }
                }
            }
            recv = source.recv() => {
                match recv {
                    Ok(tick) => {
                        accumulator.ingest(tick);
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        logging::warn(
                            "gateway.aggregator.lagged",
                            "Gateway aggregator lagged behind source ticks",
                            json!({ "skipped": skipped })
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            _ = shutdown.changed() => {
                if !matches!(*shutdown.borrow(), ShutdownSignal::None) {
                    break;
                }
            }
        }
    }

    logging::info_simple("gateway.aggregator.stop", "Gateway aggregator stopped");
    Ok(())
}

#[derive(Default)]
struct BatchAccumulator {
    latest: HashMap<String, Tick>,
}

impl BatchAccumulator {
    fn ingest(&mut self, tick: Tick) {
        self.latest.insert(tick.symbol.clone(), tick);
    }

    fn snapshot(&self) -> Vec<Tick> {
        let mut ticks: Vec<Tick> = self.latest.values().cloned().collect();
        ticks.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        ticks
    }

    fn is_empty(&self) -> bool {
        self.latest.is_empty()
    }
}

async fn run_gateway_server(
    addr: SocketAddr,
    gateway_sender: broadcast::Sender<Vec<Tick>>,
    mut shutdown: watch::Receiver<ShutdownSignal>,
) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind gateway websocket at {addr}"))?;

    logging::info(
        "gateway.bind",
        "Gateway websocket listening for clients",
        json!({ "addr": addr.to_string() }),
    );

    let app = Router::new().route(
        "/ws",
        get({
            let gateway_sender = gateway_sender.clone();
            move |ws: WebSocketUpgrade| websocket_upgrade(ws, gateway_sender.clone())
        }),
    );

    let shutdown_signal = async move {
        while shutdown.changed().await.is_ok() {
            if !matches!(*shutdown.borrow(), ShutdownSignal::None) {
                break;
            }
        }
    };

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal)
        .await
        .context("gateway server terminated with error")?;

    logging::info_simple("gateway.server.stop", "Gateway websocket server stopped");
    Ok(())
}

async fn websocket_upgrade(
    ws: WebSocketUpgrade,
    gateway_sender: broadcast::Sender<Vec<Tick>>,
) -> Response {
    ws.on_upgrade(move |socket| async move {
        if let Err(err) = forward_ticks_to_client(socket, gateway_sender.clone()).await {
            logging::warn(
                "gateway.client_error",
                "Gateway websocket client ended with error",
                json!({ "error": format!("{err:?}") }),
            );
        }
    })
}

async fn forward_ticks_to_client(
    socket: WebSocket,
    gateway_sender: broadcast::Sender<Vec<Tick>>,
) -> Result<()> {
    logging::info_simple(
        "gateway.client.connected",
        "Gateway websocket client connected",
    );

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut receiver = gateway_sender.subscribe();

    let reader = tokio::spawn(async move {
        while let Some(Ok(message)) = ws_receiver.next().await {
            if matches!(message, Message::Close(_)) {
                break;
            }
        }
    });

    loop {
        match receiver.recv().await {
            Ok(batch) => {
                if batch.is_empty() {
                    continue;
                }
                let payload = serde_json::to_string(&batch).context("serialize tick payload")?;
                if ws_sender.send(Message::Text(payload)).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                logging::warn(
                    "gateway.client.lagged",
                    "Websocket client lagged gateway messages",
                    json!({ "skipped": skipped }),
                );
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }

    reader.abort();
    let _ = reader.await;
    logging::info_simple(
        "gateway.client.disconnected",
        "Gateway websocket client disconnected",
    );
    Ok(())
}
