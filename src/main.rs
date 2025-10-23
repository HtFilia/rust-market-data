use anyhow::Result;
use clap::Parser;
use rust_market_data::chart;
use rust_market_data::cli::{self, Cli};
use rust_market_data::simulator;
use rust_market_data::tail;

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command() {
        cli::Command::Run => simulator::run().await,
        cli::Command::Tail(args) => tail::run(args).await,
        cli::Command::Chart(args) => chart::run(args).await,
    }
}
