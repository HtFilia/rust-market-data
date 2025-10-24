use std::collections::HashMap;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Args;
use textplots::{Chart, Plot, Shape};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{self, Instant};

use crate::constants::SOCKET_PATH;
use crate::tick::Tick;

#[derive(Debug, Args, Clone)]
pub struct ChartArgs {
    /// Number of seconds to collect data before plotting
    #[arg(short, long, default_value_t = 30)]
    pub duration_secs: u64,

    /// Plot only the provided symbol
    #[arg(short, long)]
    pub symbol: Option<String>,

    /// Chart width in characters
    #[arg(long, default_value_t = 120)]
    pub width: u32,

    /// Chart height in characters
    #[arg(long, default_value_t = 30)]
    pub height: u32,
}

pub async fn run(args: ChartArgs) -> Result<()> {
    let duration = Duration::from_secs(args.duration_secs);
    let collected = collect_ticks(duration, args.symbol.clone()).await?;

    if collected.is_empty() {
        bail!("no ticks collected; ensure the simulator is running and emitting data");
    }

    let (symbol, points) = if let Some(symbol) = &args.symbol {
        let Some(points) = collected.get(symbol) else {
            bail!("no ticks collected for symbol {symbol}");
        };
        (symbol.clone(), points.clone())
    } else {
        collected
            .into_iter()
            .max_by_key(|(_, pts)| pts.len())
            .expect("non-empty map after earlier check")
    };

    if points.len() < 2 {
        bail!("not enough data points to render a chart");
    }

    render_chart(&symbol, points, duration, args.width, args.height);
    Ok(())
}

async fn collect_ticks(
    duration: Duration,
    symbol_filter: Option<String>,
) -> Result<HashMap<String, Vec<(f64, f64)>>> {
    let stream = UnixStream::connect(SOCKET_PATH).await.with_context(|| {
        format!(
            "failed to connect to socket {:?}; run `cargo run -- run` first",
            SOCKET_PATH
        )
    })?;

    let mut lines = BufReader::new(stream).lines();
    let deadline = Instant::now() + duration;
    let mut reference_timestamp: Option<u128> = None;
    let mut data: HashMap<String, Vec<(f64, f64)>> = HashMap::new();

    println!(
        "Collecting ticks for {}s{}...",
        duration.as_secs(),
        symbol_filter
            .as_ref()
            .map(|s| format!(" (filtering for {s})"))
            .unwrap_or_default()
    );

    loop {
        let now = Instant::now();
        let Some(remaining) = deadline.checked_duration_since(now) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }

        match time::timeout(remaining, lines.next_line()).await {
            Ok(line_result) => match line_result? {
                Some(line) => {
                    let tick: Tick = serde_json::from_str(&line)?;
                    if let Some(ref filter) = symbol_filter {
                        if filter != &tick.symbol {
                            continue;
                        }
                    }

                    let base = reference_timestamp.get_or_insert(tick.timestamp_ms);
                    let elapsed = ((tick.timestamp_ms - *base) as f64) / 1000.0;
                    data.entry(tick.symbol.clone())
                        .or_default()
                        .push((elapsed, tick.price));
                }
                None => break,
            },
            Err(_) => break,
        }
    }

    Ok(data)
}

fn render_chart(
    symbol: &str,
    mut points: Vec<(f64, f64)>,
    duration: Duration,
    width: u32,
    height: u32,
) {
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    println!(
        "Rendering chart for {symbol} ({} samples) collected over ~{}s",
        points.len(),
        duration.as_secs()
    );

    let max_time = points
        .last()
        .map(|(t, _)| *t)
        .unwrap_or(duration.as_secs_f64())
        .max(1e-3);
    let min_price = points.iter().map(|(_, p)| *p).fold(f64::INFINITY, f64::min);
    let max_price = points
        .iter()
        .map(|(_, p)| *p)
        .fold(f64::NEG_INFINITY, f64::max);
    println!("Price range: {:.4} → {:.4}", min_price, max_price);

    let samples: Vec<(f32, f32)> = points
        .into_iter()
        .map(|(t, p)| (t as f32, p as f32))
        .collect();

    let plot_width = width.max(40);
    let plot_height = height.max(10);

    Chart::new(plot_width, plot_height, 0.0, max_time as f32)
        .lineplot(&Shape::Lines(&samples))
        .display();
    println!();
}
