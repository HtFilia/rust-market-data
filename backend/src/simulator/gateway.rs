use std::collections::hash_map::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::time::{interval, MissedTickBehavior};

use crate::{constants::TICK_BATCH_VERSION, logging, tick::Tick};

use super::{
    metrics::{MetricsEvent, MetricsTx},
    ShutdownSignal,
};

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
    queue_depth: usize,
    source_sender: broadcast::Sender<Tick>,
    metrics: MetricsTx,
    shutdowns: GatewayShutdown,
) -> Result<()> {
    let (gateway_sender, _) = broadcast::channel::<Vec<Tick>>(queue_depth * 2);
    let (queue_tx, queue_rx) = mpsc::channel::<Vec<Tick>>(queue_depth);

    tokio::try_join!(
        run_gateway_aggregator(
            throttle,
            source_sender.subscribe(),
            queue_tx,
            metrics.clone(),
            shutdowns.aggregator,
        ),
        run_gateway_dispatcher(
            queue_rx,
            gateway_sender.clone(),
            metrics.clone(),
            shutdowns.dispatcher,
        ),
        run_gateway_server(addr, gateway_sender, metrics, shutdowns.server),
    )?;

    Ok(())
}

async fn run_gateway_aggregator(
    throttle: Duration,
    mut source: broadcast::Receiver<Tick>,
    queue_sender: mpsc::Sender<Vec<Tick>>,
    metrics: MetricsTx,
    mut shutdown: watch::Receiver<ShutdownSignal>,
) -> Result<()> {
    logging::info_simple("gateway.aggregator.start", "Gateway aggregator started");

    let mut accumulator = BatchAccumulator::default();
    let mut ticker = interval(throttle);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker.reset();
    let mut lag_tracker = RateTracker::new(Duration::from_secs(1));
    let mut drop_tracker = RateTracker::new(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if !accumulator.is_empty() {
                    let snapshot = accumulator.snapshot();
                    if !snapshot.is_empty() {
                        match queue_sender.try_send(snapshot) {
                            Ok(_) => {}
                            Err(mpsc::error::TrySendError::Full(_)) => {
                                metrics.report(MetricsEvent::GatewayBackpressure { dropped: 1 });
                                if let Some((total, _)) = drop_tracker.record(1) {
                                    logging::warn(
                                        "gateway.queue.full",
                                        "Gateway queue saturated, dropping batches",
                                        json!({ "dropped_batches": total })
                                    );
                                }
                            }
                            Err(mpsc::error::TrySendError::Closed(_)) => {
                                break;
                            }
                        }
                    }
                }
            }
            recv = source.recv() => {
                match recv {
                    Ok(tick) => {
                        accumulator.ingest(tick);
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        metrics.report(MetricsEvent::GatewayLag {
                            skipped: skipped as usize,
                            component: "aggregator",
                        });
                        if let Some((total, max)) = lag_tracker.record(skipped as usize) {
                            logging::warn(
                                "gateway.aggregator.lagged",
                                "Gateway aggregator lagged behind source ticks",
                                json!({ "skipped_total": total, "max_skipped": max })
                            );
                        }
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

    if let Some((total, max)) = lag_tracker.flush() {
        logging::warn(
            "gateway.aggregator.lagged",
            "Gateway aggregator lagged behind source ticks",
            json!({ "skipped_total": total, "max_skipped": max }),
        );
    }

    if let Some((total, _)) = drop_tracker.flush() {
        logging::warn(
            "gateway.queue.full",
            "Gateway queue saturated, dropping batches",
            json!({ "dropped_batches": total }),
        );
    }

    logging::info_simple("gateway.aggregator.stop", "Gateway aggregator stopped");
    Ok(())
}

pub(super) struct GatewayShutdown {
    pub aggregator: watch::Receiver<ShutdownSignal>,
    pub dispatcher: watch::Receiver<ShutdownSignal>,
    pub server: watch::Receiver<ShutdownSignal>,
}

#[derive(Serialize)]
struct TickBatchPayload {
    version: u32,
    ticks: Vec<Tick>,
}

struct RateTracker {
    total: usize,
    max: usize,
    window: Duration,
    last_emit: Option<Instant>,
}

impl RateTracker {
    fn new(window: Duration) -> Self {
        Self {
            total: 0,
            max: 0,
            window,
            last_emit: None,
        }
    }

    fn record(&mut self, value: usize) -> Option<(usize, usize)> {
        self.total = self.total.saturating_add(value);
        self.max = self.max.max(value);
        let now = Instant::now();
        match self.last_emit {
            Some(last) if now.duration_since(last) >= self.window => {
                self.last_emit = Some(now);
                let total = std::mem::take(&mut self.total);
                let max = std::mem::take(&mut self.max);
                Some((total, max))
            }
            None => {
                self.last_emit = Some(now);
                None
            }
            _ => None,
        }
    }

    fn flush(&mut self) -> Option<(usize, usize)> {
        if self.total > 0 {
            let total = std::mem::take(&mut self.total);
            let max = std::mem::take(&mut self.max);
            self.last_emit = Some(Instant::now());
            Some((total, max))
        } else {
            None
        }
    }
}

async fn run_gateway_dispatcher(
    mut queue: mpsc::Receiver<Vec<Tick>>,
    gateway_sender: broadcast::Sender<Vec<Tick>>,
    metrics: MetricsTx,
    mut shutdown: watch::Receiver<ShutdownSignal>,
) -> Result<()> {
    logging::info_simple("gateway.dispatcher.start", "Gateway dispatcher started");

    loop {
        tokio::select! {
            batch = queue.recv() => {
                match batch {
                    Some(batch) => {
                        metrics.report(MetricsEvent::GatewayBatch { symbols: batch.len() });
                        let _ = gateway_sender.send(batch);
                    }
                    None => break,
                }
            }
            _ = shutdown.changed() => {
                if !matches!(*shutdown.borrow(), ShutdownSignal::None) {
                    break;
                }
            }
        }
    }

    logging::info_simple("gateway.dispatcher.stop", "Gateway dispatcher stopped");
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
    metrics: MetricsTx,
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
            let metrics = metrics.clone();
            move |ws: WebSocketUpgrade| {
                websocket_upgrade(ws, gateway_sender.clone(), metrics.clone())
            }
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
    metrics: MetricsTx,
) -> Response {
    ws.on_upgrade(move |socket| async move {
        if let Err(err) =
            forward_ticks_to_client(socket, gateway_sender.clone(), metrics.clone()).await
        {
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
    metrics: MetricsTx,
) -> Result<()> {
    logging::info_simple(
        "gateway.client.connected",
        "Gateway websocket client connected",
    );

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut receiver = gateway_sender.subscribe();
    let mut lag_tracker = RateTracker::new(Duration::from_secs(1));

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
                let payload = serde_json::to_string(&TickBatchPayload {
                    version: TICK_BATCH_VERSION,
                    ticks: batch,
                })
                .context("serialize tick payload")?;
                if ws_sender.send(Message::Text(payload)).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                metrics.report(MetricsEvent::GatewayLag {
                    skipped: skipped as usize,
                    component: "client",
                });
                if let Some((total, max)) = lag_tracker.record(skipped as usize) {
                    logging::warn(
                        "gateway.client.lagged",
                        "Websocket client lagged gateway messages",
                        json!({ "skipped_total": total, "max_skipped": max }),
                    );
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }

    reader.abort();
    let _ = reader.await;

    if let Some((total, max)) = lag_tracker.flush() {
        logging::warn(
            "gateway.client.lagged",
            "Websocket client lagged gateway messages",
            json!({ "skipped_total": total, "max_skipped": max }),
        );
    }

    logging::info_simple(
        "gateway.client.disconnected",
        "Gateway websocket client disconnected",
    );
    Ok(())
}
