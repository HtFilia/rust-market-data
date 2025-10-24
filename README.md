# Rust Market Data Simulator

This crate generates correlated market data ticks for a sizeable universe of equities (500 names) and streams them to a Unix domain socket. It is intended as a learning aid for exploring idiomatic Rust structures such as async tasks, shared state management, linear algebra with `nalgebra`, and serialization via `serde`.

## Features

- Synthesises a 500-name equity universe with geography and sector metadata.
- Generates a positive-definite correlation matrix using factor loadings with regional and sector rules, refreshing it periodically.
- Samples correlated Gaussian returns at a high cadence (default 8ms) to evolve prices.
- Publishes JSON-encoded ticks over a Unix socket so external processes can subscribe in real time.
- Emits structured JSON logs suitable for downstream ingestion pipelines.

## Usage

### Run the simulator

```bash
cargo run
```

The process binds to `market_ticks.sock` inside the project directory. The other subcommands expect that socket to be available.

The runtime responds to common Unix signals when running the simulator:

- `SIGTERM` performs a graceful shutdown, letting background tasks finish and removing the socket file.
- `SIGHUP` triggers a hot reload of the correlation structure.
- `SIGINT` (Ctrl+C) exits immediately after cleaning up the socket.

### Inspect ticks in real time

```bash
cargo run -- tail
```

Use `--symbol AAPL` to filter to a single instrument or `--limit 20` to stop after a fixed number of ticks.

### Visualise a price path

Collect ticks for 30 seconds (configurable) and render an ASCII chart for the most active symbol, or provide `--symbol` to select one explicitly:

```bash
cargo run -- chart --duration 45 --symbol AAPL
```

This is useful for getting an intuition for the geometric Brownian motion driving prices.

## Socket payload format

Any process can subscribe by opening the socket and reading newline-delimited JSON. For example:

```bash
# In another terminal:
nc -U market_ticks.sock
```

Each line contains a JSON payload of the form:

```json
{"symbol":"NA_TECH000","price":101.234,"timestamp_ms":1716400000000,"region":"north_america","sector":"technology"}
```

## Customisation

- Update the sector/region mix or instrumentation in `src/model.rs` if you want a different default universe.
- Adjust constants in `src/constants.rs` (e.g. tick cadence or refresh period) to suit different sampling speeds.
- Tweak the price step sizing or correlation factor rules in `src/simulator/universe.rs` if you want alternative dynamics.

## Code layout

- `src/cli.rs` wires the Clap-based command-line interface.
- `src/simulator/` hosts the core market model (universe construction, tick loop, socket server).
- `src/tail.rs` and `src/chart.rs` implement the inspection utilities that subscribe to the Unix socket.
- `src/tick.rs` and `src/constants.rs` capture shared data types and configuration.
- `schemas/` contains JSON Schemas and example payloads for ticks and structured logs.
