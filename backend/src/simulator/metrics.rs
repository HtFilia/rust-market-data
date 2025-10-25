use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use serde_json::{json, Map, Value};
use tokio::{
    sync::{mpsc, watch},
    time::{interval, MissedTickBehavior},
};

use crate::logging;

use super::ShutdownSignal;

#[derive(Debug)]
pub enum MetricsEvent {
    TickBatch {
        generated: usize,
    },
    GatewayBatch {
        symbols: usize,
    },
    GatewayLag {
        skipped: usize,
        component: &'static str,
    },
}

#[derive(Clone, Default)]
pub struct MetricsTx(Option<mpsc::UnboundedSender<MetricsEvent>>);

impl MetricsTx {
    pub fn report(&self, event: MetricsEvent) {
        if let Some(sender) = &self.0 {
            let _ = sender.send(event);
        }
    }

    pub fn noop() -> Self {
        Self(None)
    }
}

pub fn reporter(
    shutdown: watch::Receiver<ShutdownSignal>,
) -> (MetricsTx, impl std::future::Future<Output = Result<()>>) {
    let (tx, rx) = mpsc::unbounded_channel();
    (MetricsTx(Some(tx)), process_events(rx, shutdown))
}

async fn process_events(
    mut rx: mpsc::UnboundedReceiver<MetricsEvent>,
    mut shutdown: watch::Receiver<ShutdownSignal>,
) -> Result<()> {
    let mut tick_batches: usize = 0;
    let mut total_ticks: usize = 0;
    let mut gateway_batches: usize = 0;
    let mut gateway_symbols: usize = 0;
    let mut gateway_max_batch: usize = 0;
    let mut gateway_lag: HashMap<&'static str, (usize, usize)> = HashMap::new();

    let mut reporter = interval(Duration::from_secs(1));
    reporter.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(MetricsEvent::TickBatch { generated }) => {
                        tick_batches = tick_batches.saturating_add(1);
                        total_ticks = total_ticks.saturating_add(generated);
                    }
                    Some(MetricsEvent::GatewayBatch { symbols }) => {
                        gateway_batches = gateway_batches.saturating_add(1);
                        gateway_symbols = gateway_symbols.saturating_add(symbols);
                        gateway_max_batch = gateway_max_batch.max(symbols);
                    }
                    Some(MetricsEvent::GatewayLag { skipped, component }) => {
                        let entry = gateway_lag.entry(component).or_insert((0, 0));
                        entry.0 = entry.0.saturating_add(1);
                        entry.1 = entry.1.saturating_add(skipped);
                    }
                    None => break,
                }
            }
            _ = reporter.tick() => {
                if tick_batches > 0 || gateway_batches > 0 || !gateway_lag.is_empty() {
                    let lag_snapshot = if gateway_lag.is_empty() {
                        Value::Null
                    } else {
                        let mut map = Map::new();
                        for (component, (events, skipped)) in &gateway_lag {
                            map.insert(
                                component.to_string(),
                                json!({
                                    "events": events,
                                    "skipped": skipped
                                }),
                            );
                        }
                        Value::Object(map)
                    };

                    logging::info(
                        "metrics.throughput",
                        "tick throughput summary",
                        json!({
                            "tick_batches": tick_batches,
                            "total_ticks": total_ticks,
                            "avg_ticks_per_batch": if tick_batches > 0 { total_ticks as f64 / tick_batches as f64 } else { 0.0 },
                            "gateway_batches": gateway_batches,
                            "avg_gateway_symbols": if gateway_batches > 0 { gateway_symbols as f64 / gateway_batches as f64 } else { 0.0 },
                            "gateway_max_symbols": gateway_max_batch,
                            "gateway_lag": lag_snapshot,
                        })
                    );
                }

                tick_batches = 0;
                total_ticks = 0;
                gateway_batches = 0;
                gateway_symbols = 0;
                gateway_max_batch = 0;
                gateway_lag.clear();
            }
            changed = shutdown.changed() => {
                if changed.is_ok() && !matches!(*shutdown.borrow(), ShutdownSignal::None) {
                    break;
                }
            }
        }
    }

    logging::info_simple("metrics.stop", "Metrics reporter stopped");
    Ok(())
}
