# Agent Guide

Welcome to the `rust-market-data` workspace. Future sessions should follow the expectations below to keep both the backend simulator and frontend UI healthy.

## Project Snapshot

- Workspace layout:
  - `backend/` – Rust async simulator (library + binary).
  - `frontend/` – Leptos client consuming websocket ticks.
  - `schemas/` – JSON Schemas + samples for interop.
- Backend generates a 500-name equity universe with `Region`/`Sector` metadata (`backend/src/model.rs`).
- Correlation matrices come from factor loadings (`backend/src/simulator/universe.rs`).
- Tick cadence defaults to 8 ms (`backend/src/constants.rs`) and every tick carries region/sector metadata.
- Runtime logging is JSON-formatted via helpers in `backend/src/logging.rs`.
- CLI entry-points live in `backend/src/cli.rs`; `backend/src/main.rs` remains a thin dispatcher.

## Daily Workflow

1. **Build & Lint**
   - `cargo fmt --all` before committing code changes.
   - `cargo clippy --workspace --all-targets -- -D warnings` (install `clippy` if missing).
2. **Tests**
   - `cargo test --workspace` exercises backend + frontend suites (backend integration uses `simulator::testkit`).
   - Add coverage when behaviour changes (e.g., new CLI flags, UI interactions).
3. **Logging**
   - Emit structured logs through `logging::info|warn|error` helpers. Never use `println!`/`eprintln!` outside those wrappers.

## Coding Notes

- Backend layout: simulator logic in `backend/src/simulator/`, data definitions in `backend/src/model.rs`, CLI-specific code in `backend/src/cli.rs`.
- Frontend layout: Reactivity + components in `frontend/src/`; keep wasm-only logic behind `cfg(target_arch = "wasm32")` if needed.
- Universe changes must keep the correlation matrix symmetric and SPD; rely on `StockUniverse::factor_based_correlation` or extend it carefully.
- Tick generation must remain non-blocking; avoid heavy per-tick allocations.
- Respect the existing JSON schema in `Tick`.

## Operations Tips

- Socket path defaults to `market_ticks.sock`; alter via `SimulatorConfig` if necessary.
- Signals: `SIGTERM` → graceful, `SIGHUP` → hot reload correlation, `SIGINT` → immediate stop. Behaviour is encoded in `backend/src/simulator/mod.rs`.
- For quick smoke checks without sockets use `simulator::testkit::collect_ticks`.
- Frontend development typically targets `wasm32-unknown-unknown`; use `make frontend-build` for local wasm artefacts.

## Outstanding Ideas

- CLI flags for custom universe sizes / correlation tuning.
- Metrics instrumentation (latency measurements for tick loop).
- Benchmarks to validate throughput under the 8 ms cadence target.
- End-to-end tests wiring websocket ticks into the frontend.

Please update this file whenever the workflow or expectations change so the next agent has accurate context.
