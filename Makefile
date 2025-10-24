CARGO_BIN := cargo
HOOKS_PATH := .githooks

.PHONY: help install build run tail chart test clean fmt lint bench frontend-build frontend-check

help:
	@echo "Usage: make <target>"
	@echo
	@echo "Targets:"
	@echo "  help       Show this help message"
	@echo "  install    Configure Git hooks to use $(HOOKS_PATH)"
	@echo "  build      Build all workspace crates"
	@echo "  run        Run the backend simulator (tick generator + socket server)"
	@echo "  tail       Subscribe to ticks via the backend CLI tail command"
	@echo "  chart      Render a price chart using the backend CLI chart command"
	@echo "  test       Run the full test suite"
	@echo "  fmt        Format all workspace code with rustfmt"
	@echo "  lint       Run clippy with warnings treated as errors"
	@echo "  bench      Execute cargo bench"
	@echo "  frontend-build  Build the frontend (wasm32-unknown-unknown)"
	@echo "  frontend-check  cargo check frontend crate"
	@echo "  clean      Remove build artifacts"

install:
	@echo "Configuring git hooks path -> $(HOOKS_PATH)"
	git config core.hooksPath $(HOOKS_PATH)
	@echo "Hooks installed."

build:
	$(CARGO_BIN) build --workspace

run:
	$(CARGO_BIN) run -p rust-market-data -- run

tail:
	$(CARGO_BIN) run -p rust-market-data -- tail

chart:
	$(CARGO_BIN) run -p rust-market-data -- chart

test:
	$(CARGO_BIN) test --workspace

fmt:
	$(CARGO_BIN) fmt --all

lint:
	$(CARGO_BIN) clippy --workspace --all-targets -- -D warnings

bench:
	$(CARGO_BIN) bench

frontend-build:
	$(CARGO_BIN) build -p frontend --target wasm32-unknown-unknown

frontend-check:
	$(CARGO_BIN) check -p frontend

clean:
	$(CARGO_BIN) clean
