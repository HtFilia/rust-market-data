# Agent Guide

Welcome to the `rust-market-data` project. Future sessions should follow the expectations below to keep the simulator consistent and healthy.

## Project Snapshot

- Generates a 500-name equity universe tagged with `Region` and `Sector` enums defined in `src/model.rs`.
- Correlation matrices come from factor loadings; see `src/simulator/universe.rs`.
- Tick cadence defaults to 5 ms (`src/constants.rs`) and every tick carries region/sector metadata.
- Runtime logging is JSON-formatted via helpers in `src/logging.rs`.
- CLI entry-points live in `src/cli.rs`; `src/main.rs` is a thin dispatcher into the library.

## Daily Workflow

1. **Build & Lint**
   - `cargo fmt` before committing code changes.
   - `cargo clippy --all-targets -- -D warnings` (install `clippy` if missing).
2. **Tests**
   - `cargo test` exercises unit + integration suites (async integration uses `simulator::testkit`).
   - Add coverage when behaviour changes (e.g. more factor rules, CLI flags).
3. **Logging**
   - Emit structured logs through `logging::info|warn|error` helpers. Never use `println!`/`eprintln!` outside those wrappers.

## Coding Notes

- Stick to the modular layout: simulator logic in `src/simulator/`, data definitions in `src/model.rs`, CLI-specific code in `src/cli.rs`.
- Universe changes must keep the correlation matrix symmetric and SPD; rely on `StockUniverse::factor_based_correlation` or extend it carefully.
- Tick generation must remain non-blocking; avoid heavy per-tick allocations.
- Respect the existing JSON schema in `Tick`.

## Operations Tips

- Socket path defaults to `market_ticks.sock`; alter via `SimulatorConfig` if necessary.
- Signals: `SIGTERM` → graceful, `SIGHUP` → hot reload correlation, `SIGINT` → immediate stop. Behaviour is encoded in `src/simulator/mod.rs`.
- For quick smoke checks without sockets use `simulator::testkit::collect_ticks`.

## Outstanding Ideas

- CLI flags for custom universe sizes / correlation tuning.
- Metrics instrumentation (latency measurements for tick loop).
- Benchmarks to validate throughput under 5 ms cadence.

Please update this file whenever the workflow or expectations change so the next agent has accurate context.
