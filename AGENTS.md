# Agent Guide

Welcome to the `rust-market-data` workspace. This document is the single source of truth for future automation agents. Keep it current so new contributors can come up to speed without trawling the repo history.

## 1. Business Context & Current Capabilities

The project simulates a high-frequency equity market and renders it in a browser dashboard. Typical use cases are live dashboard demos, load-testing downstream consumers, and experimentation with async Rust patterns.

- **Backend (Rust)**
  - Generates a 500-name universe with sector/region metadata (`backend/src/model.rs`).
  - Synthesises correlated price paths via factor loadings (`backend/src/simulator/universe.rs`).
  - Streams newline-delimited JSON ticks over a Unix socket and emits throttled websocket batches (one snapshot per second) with versioned payloads (`backend/src/simulator/gateway.rs`).
  - Provides JSON logging helpers (`backend/src/logging.rs`) and CLI utilities (`run`, `tail`, `chart`).
  - Observability: periodic throughput metrics, lag/backpressure tracking, graceful signal handling.

- **Frontend (Rust → WASM via Leptos)**
  - Connects to the websocket gateway with automatic reconnect/backoff and status reporting (`frontend/src/ticks/websocket.rs`).
  - Displays live quotes, a history chart, summary movers, and filter controls for region/sector (`frontend/src/components/*`).
  - Supports Dark/Light/Sepia themes via CSS custom properties.

- **Interoperability**
  - JSON Schemas for ticks, batches, and logs live in `/schemas`. Payloads are versioned (`version: 1`) to keep future changes explicit.
  - End-to-end websocket integration test ensures simulator → gateway → client contract stability (`backend/tests/e2e_realtime.rs`).

## 2. Tech Stack Overview

| Layer        | Tools / Libraries                       | Notes |
|--------------|----------------------------------------|-------|
| Runtime      | Rust 1.78+, Tokio, Rayon, Axum (ws)    | Async multi-task execution, websocket server |
| Math         | `nalgebra`, `rand`, `rand_distr`       | Correlated price generation |
| Frontend WASM| Leptos 0.6 (CSR), gloo-net/timers, wasm-bindgen | Reactive components, websocket client |
| Testing      | Cargo test, tokio-tungstenite, Playwright TBD | Integration + unit coverage |
| Tooling      | git hooks (fmt/clippy/test), Makefile  | Automation of quality gates |

## 3. Development Workflow & Practices

1. **Quality Gates** (enforced by hooks)
   - `cargo fmt --all`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`

2. **Testing Strategy**
   - Unit tests for tick store, chart geometry, websocket deserialisation, simulator universe, etc.
   - Integration tests using `simulator::testkit` for backend behaviour.
   - End-to-end websocket test (`backend/tests/e2e_realtime.rs`) to guarantee batches stream as expected. Future iterations should extend this to browser-level harnesses when Playwright setup is available.

3. **Coding Guidelines**
   - Keep tick generation hot path allocation-free; reuse buffers where possible.
   - Maintain SPD correlation matrices—do not shortcut the `StockUniverse` rebuild logic.
   - All logs must go through `logging::*` helpers; avoid `println!`/`eprintln!`.
   - Web payloads must adhere to the schemas in `/schemas`; bump `TICK_BATCH_VERSION` if the contract changes.
   - Frontend wasm-only code must be `cfg(target_arch = "wasm32")` gated.

4. **TDD Expectations**
   - Add or update tests whenever behaviour changes (e.g., new UI state, CLI option, payload structure).
   - Use focused unit tests for pure logic (e.g., movers calculation) and integration tests for async pipelines.
   - Prefer writing the failing test first, then the implementation; remove any flaky async sleeps by relying on deterministic utilities (timeouts, polling).

## 4. Architecture Cheat Sheet

- `backend/src/simulator/mod.rs`: orchestrates tick generator, gateway queue/dispatcher, metrics reporter, and signal handling.
- `backend/src/simulator/gateway.rs`: websocket batching, rate-limited lag/backpressure logging, queue fanout.
- `frontend/src/components`: `dashboard.rs` wires state; `tick_table.rs`, `summary.rs`, `filters.rs`, `history_chart.rs` compose the UI.
- `frontend/src/ticks/websocket.rs`: websocket client with exponential backoff and status callbacks.
- `frontend/style.css`: theme palette definitions (dark/light/sepia) and layout (two-column with sticky sidebar).

## 5. Running & Troubleshooting

- Start backend: `make run` (or `cargo run -p rust-market-data -- run`).
- Start frontend (during development): use `trunk serve` in `frontend/` after building.
- Inspect ticks via CLI: `make tail` or `cargo run -p rust-market-data -- tail --symbol NAHLT000`.
- If the websocket dashboard shows “Disconnected”, check backend logs for `gateway.*` events. Lag warnings are rate-limited; persistent drops indicate the queue depth or throttle may need tuning.
- End-to-end test smoke: `cargo test --package rust-market-data --test e2e_realtime`.

## 6. Near-Term Roadmap / Ideas

- Expand e2e coverage with Playwright once installable (smoke filters, theme switch, reconnection UX).
- Expose metrics over HTTP (Prometheus or JSON) for external monitoring.
- Configurable universe sizing and correlation knobs via CLI flags and config files.
- Benchmark suite to profile throughput under different throttles.
- Optional historical persistence (e.g., to disk or SQLite) for replay.

Keep this guide updated whenever architecture, workflows, or expectations shift. A clear AGENTS.md keeps future runs efficient and avoids repeating discovery work.
