use clap::{Parser, Subcommand};

use crate::chart::ChartArgs;
use crate::tail::TailArgs;

#[derive(Debug, Parser)]
#[command(author, version, about = "Correlated market data simulator")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    pub fn command(self) -> Command {
        self.command.unwrap_or_default()
    }
}

#[derive(Debug, Subcommand, Default)]
pub enum Command {
    /// Run the tick generator and socket publisher
    #[default]
    Run,
    /// Subscribe to the unix socket and print incoming ticks
    Tail(TailArgs),
    /// Collect ticks and render an ASCII price chart
    Chart(ChartArgs),
}
