use anyhow::{Context, Result};
use clap::Args;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;

use crate::constants::SOCKET_PATH;
use crate::tick::Tick;

#[derive(Debug, Args, Clone)]
pub struct TailArgs {
    /// Filter ticks to a single symbol (e.g. AAPL)
    #[arg(short, long)]
    pub symbol: Option<String>,

    /// Stop after printing this many ticks
    #[arg(short, long)]
    pub limit: Option<usize>,
}

pub async fn run(args: TailArgs) -> Result<()> {
    let stream = UnixStream::connect(SOCKET_PATH).await.with_context(|| {
        format!(
            "failed to connect to socket {:?}; run `cargo run -- run` first",
            SOCKET_PATH
        )
    })?;

    let mut lines = BufReader::new(stream).lines();
    let mut printed = 0usize;
    println!("Connected to {SOCKET_PATH}; streaming ticks...");

    while let Some(line) = lines.next_line().await? {
        let tick: Tick = serde_json::from_str(&line)?;
        if let Some(ref filter) = args.symbol {
            if filter != &tick.symbol {
                continue;
            }
        }

        println!(
            "{:>16} | {:>12} | {:>8.4} | {:>18} | {:>22}",
            tick.timestamp_ms, tick.symbol, tick.price, tick.region, tick.sector
        );
        printed += 1;

        if let Some(limit) = args.limit {
            if printed >= limit {
                break;
            }
        }
    }
    Ok(())
}
