# Rust Market Data Workspace

This repository hosts a two-crate Cargo workspace:

- `backend/` – the async market data simulator (original project moved here).
- `frontend/` – a Leptos-based UI that consumes the simulator ticks.

Most backend-specific documentation (tick schema, CLI usage, hooks, Make targets) lives in `backend/README.md`.

## Getting Started

```bash
make install   # configure git hooks
make build     # build both crates
make run       # launch the backend simulator
make frontend-build   # build the frontend wasm output
```

See `Makefile` for the full list of helper targets.

> The frontend build assumes the `wasm32-unknown-unknown` target is installed: `rustup target add wasm32-unknown-unknown`.

Schemas documenting tick/log payloads are under `schemas/`.
