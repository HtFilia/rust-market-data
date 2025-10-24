mod universe;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{broadcast, watch, RwLock};
use tokio::time::{self, MissedTickBehavior};

use crate::constants::{CORRELATION_REFRESH_SECS, SOCKET_PATH, TICK_INTERVAL_MS};
use crate::logging;
use crate::model::default_equities;
use crate::tick::Tick;

use universe::StockUniverse;

#[derive(Clone, Debug)]
pub struct SimulatorConfig {
    pub socket_path: PathBuf,
    pub tick_interval: Duration,
    pub correlation_refresh: Duration,
    pub max_ticks: Option<usize>,
    pub enable_socket: bool,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from(SOCKET_PATH),
            tick_interval: Duration::from_millis(TICK_INTERVAL_MS),
            correlation_refresh: Duration::from_secs(CORRELATION_REFRESH_SECS),
            max_ticks: None,
            enable_socket: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShutdownSignal {
    None,
    Graceful,
    Immediate,
}

pub async fn run() -> Result<()> {
    run_with_config(SimulatorConfig::default()).await
}

pub async fn run_with_config(config: SimulatorConfig) -> Result<()> {
    let config = Arc::new(config);

    let mut rng = StdRng::from_entropy();
    let equities = default_equities();
    let initial_prices: Vec<f64> = equities
        .iter()
        .map(|_| rng.gen_range(80.0..150.0))
        .collect();
    let universe = Arc::new(RwLock::new(StockUniverse::new(equities, &mut rng)?));

    let (shutdown_tx, shutdown_rx) = watch::channel(ShutdownSignal::None);
    let (reload_tx, _) = broadcast::channel::<()>(16);

    let (tick_sender, _) = broadcast::channel::<Tick>(4096);
    let server_sender = tick_sender.clone();

    let signals_task = tokio::spawn(handle_signals(shutdown_tx.clone(), reload_tx.clone()));

    let shutdown_for_socket = shutdown_rx.clone();
    let shutdown_for_ticks = shutdown_rx.clone();
    let shutdown_for_corr = shutdown_rx;

    let socket_future = async {
        if config.enable_socket {
            run_socket_server(Arc::clone(&config), server_sender, shutdown_for_socket).await
        } else {
            Ok(())
        }
    };

    let run_result = tokio::try_join!(
        socket_future,
        run_tick_generator(
            Arc::clone(&config),
            Arc::clone(&universe),
            initial_prices,
            tick_sender,
            shutdown_tx.clone(),
            shutdown_for_ticks
        ),
        run_correlation_updates(
            Arc::clone(&config),
            Arc::clone(&universe),
            shutdown_for_corr,
            reload_tx.subscribe()
        )
    );

    signals_task.abort();
    let _ = signals_task.await;

    run_result?;
    Ok(())
}

async fn handle_signals(
    shutdown_tx: watch::Sender<ShutdownSignal>,
    reload_tx: broadcast::Sender<()>,
) -> Result<()> {
    let mut sigterm =
        signal(SignalKind::terminate()).context("failed to register SIGTERM handler")?;
    let mut sigint =
        signal(SignalKind::interrupt()).context("failed to register SIGINT handler")?;
    let mut sighup = signal(SignalKind::hangup()).context("failed to register SIGHUP handler")?;

    loop {
        tokio::select! {
            _ = sigterm.recv() => {
                logging::info(
                    "signal.received",
                    "SIGTERM received, initiating graceful shutdown",
                    json!({ "signal": "SIGTERM" })
                );
                if shutdown_tx.send(ShutdownSignal::Graceful).is_err() {
                    break;
                }
            }
            _ = sigint.recv() => {
                logging::warn(
                    "signal.received",
                    "SIGINT received, forcing immediate shutdown",
                    json!({ "signal": "SIGINT" })
                );
                let _ = shutdown_tx.send(ShutdownSignal::Immediate);
                break;
            }
            _ = sighup.recv() => {
                logging::info(
                    "signal.received",
                    "SIGHUP received, triggering hot reload",
                    json!({ "signal": "SIGHUP" })
                );
                let _ = reload_tx.send(());
            }
        }
    }

    Ok(())
}

async fn run_tick_generator(
    config: Arc<SimulatorConfig>,
    universe: Arc<RwLock<StockUniverse>>,
    mut prices: Vec<f64>,
    sender: broadcast::Sender<Tick>,
    shutdown_tx: watch::Sender<ShutdownSignal>,
    mut shutdown_rx: watch::Receiver<ShutdownSignal>,
) -> Result<()> {
    use nalgebra::DVector;
    use rand_distr::StandardNormal;

    let mut rng = StdRng::from_entropy();
    let tick_interval = config.tick_interval;
    let max_ticks = config.max_ticks;

    let mut ticker = time::interval(tick_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let equities = {
        let guard = universe.read().await;
        guard.equities().to_vec()
    };
    let mut emitted_ticks: usize = 0;

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = shutdown_rx.changed() => {
                match *shutdown_rx.borrow() {
                    ShutdownSignal::None => continue,
                    _ => break,
                }
            }
        }

        let cholesky = {
            let guard = universe.read().await;
            guard.cholesky().clone()
        };

        let dim = cholesky.nrows();
        let mut draws = DVector::zeros(dim);
        for i in 0..dim {
            draws[i] = rng.sample(StandardNormal);
        }
        let correlated = &cholesky * draws;
        let correlated_slice = correlated.as_slice();
        let timestamp_base = current_timestamp_ms();

        let ticks: Vec<Tick> = prices
            .par_iter_mut()
            .zip(equities.par_iter())
            .zip(correlated_slice.par_iter())
            .enumerate()
            .map(|(idx, ((price, equity), corr))| {
                *price = (*price * (1.0 + *corr * 0.002)).max(0.01);
                Tick {
                    symbol: equity.symbol.clone(),
                    price: *price,
                    timestamp_ms: timestamp_base + idx as u128,
                    region: equity.region,
                    sector: equity.sector,
                }
            })
            .collect();

        emitted_ticks = emitted_ticks.saturating_add(ticks.len());
        for tick in ticks {
            let _ = sender.send(tick);
        }

        if let Some(max) = max_ticks {
            if emitted_ticks >= max {
                logging::info(
                    "tick_generator.limit",
                    "Tick generator reached max tick budget",
                    json!({ "max_ticks": max }),
                );
                let _ = shutdown_tx.send(ShutdownSignal::Graceful);
                break;
            }
        }
    }

    logging::info_simple("tick_generator.stop", "Tick generator stopped");
    Ok(())
}

async fn run_correlation_updates(
    config: Arc<SimulatorConfig>,
    universe: Arc<RwLock<StockUniverse>>,
    mut shutdown: watch::Receiver<ShutdownSignal>,
    mut reload_rx: broadcast::Receiver<()>,
) -> Result<()> {
    let mut rng = StdRng::from_entropy();
    let refresh_period = config.correlation_refresh;

    loop {
        tokio::select! {
            _ = time::sleep(refresh_period) => {
                let mut guard = universe.write().await;
                guard.refresh(&mut rng)?;
                logging::info_simple("correlation.refresh", "Correlation matrix refreshed");
            }
            recv = reload_rx.recv() => {
                match recv {
                    Ok(_) => {
                        let mut guard = universe.write().await;
                        guard.rebuild(&mut rng)?;
                        logging::info_simple("correlation.reload", "Correlation matrix hot reloaded");
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = shutdown.changed() => {
                if matches!(*shutdown.borrow(), ShutdownSignal::None) {
                    continue;
                }
                break;
            }
        }
    }

    logging::info_simple("correlation.stop", "Correlation updater stopped");
    Ok(())
}

async fn run_socket_server(
    config: Arc<SimulatorConfig>,
    sender: broadcast::Sender<Tick>,
    mut shutdown: watch::Receiver<ShutdownSignal>,
) -> Result<()> {
    let socket_path = config.socket_path.clone();
    cleanup_socket_path(&socket_path)?;
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("failed to bind unix socket at {:?}", socket_path))?;
    logging::info(
        "socket.bind",
        "Listening for tick subscribers",
        json!({ "path": socket_path.display().to_string() }),
    );

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (stream, _) = accept_result?;
                let mut receiver = sender.subscribe();
                tokio::spawn(async move {
                    if let Err(err) = forward_ticks_to_client(stream, &mut receiver).await {
                        logging::warn(
                            "socket.stream_error",
                            "Tick stream task ended with error",
                            json!({ "error": format!("{err:?}") })
                        );
                    }
                });
            }
            _ = shutdown.changed() => {
                match *shutdown.borrow() {
                    ShutdownSignal::None => continue,
                    ShutdownSignal::Graceful => {
                        logging::info_simple("socket.shutdown", "Socket server shutting down gracefully");
                        break;
                    }
                    ShutdownSignal::Immediate => {
                        logging::warn_simple("socket.shutdown", "Socket server stopping immediately");
                        break;
                    }
                }
            }
        }
    }

    drop(sender);
    cleanup_socket_path(&socket_path)?;
    logging::info(
        "socket.cleanup",
        "Socket removed after shutdown",
        json!({ "path": socket_path.display().to_string() }),
    );
    Ok(())
}

async fn forward_ticks_to_client(
    mut stream: UnixStream,
    receiver: &mut broadcast::Receiver<Tick>,
) -> Result<()> {
    loop {
        match receiver.recv().await {
            Ok(tick) => {
                let payload = serde_json::to_vec(&tick)?;
                if let Err(err) = stream.write_all(&payload).await {
                    if is_disconnect(&err) {
                        logging::info(
                            "socket.client_disconnect",
                            "Tick subscriber disconnected during payload write",
                            json!({ "reason": err.kind().to_string() }),
                        );
                        break;
                    }
                    return Err(err.into());
                }
                if let Err(err) = stream.write_all(b"\n").await {
                    if is_disconnect(&err) {
                        logging::info(
                            "socket.client_disconnect",
                            "Tick subscriber disconnected during newline write",
                            json!({ "reason": err.kind().to_string() }),
                        );
                        break;
                    }
                    return Err(err.into());
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                logging::warn(
                    "socket.lagged",
                    "Subscriber lagged tick messages",
                    json!({ "skipped": skipped }),
                );
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    Ok(())
}

fn is_disconnect(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        ErrorKind::BrokenPipe | ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
    )
}

fn cleanup_socket_path(socket_path: &Path) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)
            .with_context(|| format!("failed to remove old socket at {:?}", socket_path))?;
    }
    Ok(())
}

fn current_timestamp_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis()
}

pub mod testkit {
    use super::*;
    use rand::SeedableRng;

    pub async fn collect_ticks(mut config: SimulatorConfig, count: usize) -> Result<Vec<Tick>> {
        config.enable_socket = false;
        config.max_ticks = None;

        let config = Arc::new(config);
        let mut rng = StdRng::seed_from_u64(0xBADF00D);
        let equities = default_equities();
        let initial_prices: Vec<f64> = equities
            .iter()
            .map(|_| rng.gen_range(80.0..150.0))
            .collect();
        let universe = Arc::new(RwLock::new(StockUniverse::new(equities, &mut rng)?));

        let (shutdown_tx, shutdown_rx) = watch::channel(ShutdownSignal::None);
        let (reload_tx, _) = broadcast::channel::<()>(1);
        let (tick_sender, _) = broadcast::channel::<Tick>(4096);
        let mut receiver = tick_sender.subscribe();

        let generator_handle = tokio::spawn(run_tick_generator(
            Arc::clone(&config),
            Arc::clone(&universe),
            initial_prices,
            tick_sender,
            shutdown_tx.clone(),
            shutdown_rx.clone(),
        ));

        let correlation_handle = tokio::spawn(run_correlation_updates(
            Arc::clone(&config),
            Arc::clone(&universe),
            shutdown_rx,
            reload_tx.subscribe(),
        ));

        let mut collected = Vec::with_capacity(count);
        while collected.len() < count {
            let tick = receiver.recv().await?;
            collected.push(tick);
        }

        let _ = shutdown_tx.send(ShutdownSignal::Graceful);
        let _ = generator_handle.await??;
        let _ = correlation_handle.await??;

        Ok(collected)
    }
}
