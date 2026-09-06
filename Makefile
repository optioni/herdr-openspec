.PHONY: build fmt fmt-check lint test coverage gates gates-full check

build:
	/bin/sh scripts/build.sh

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	@if ! cargo clippy --version >/dev/null 2>&1; then \
		echo "error: clippy not found. Install it with: rustup component add clippy" 1>&2; \
		exit 1; \
	fi
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all-features

coverage:
	@if ! cargo llvm-cov --version >/dev/null 2>&1; then \
		echo "error: cargo-llvm-cov not found. Install it with: cargo install cargo-llvm-cov" 1>&2; \
		exit 1; \
	fi
	cargo llvm-cov --fail-under-lines 80

gates:
	/bin/sh scripts/gates/deps.sh
	env -u GRAPH_WRITE /bin/sh scripts/gates/build-graph.sh
	/bin/sh scripts/gates/wired.sh
	/bin/sh scripts/gates/agentseam.sh
	/bin/sh scripts/gates/launchseam.sh
	LAUNCH=src/open.rs ENTRY='pub fn run_from_env\(' /bin/sh scripts/gates/launchseam.sh
	/bin/sh scripts/gates/detailwidths.sh
	/bin/sh scripts/gates/listwidths.sh
	/bin/sh scripts/gates/mdseam.sh
	/bin/sh scripts/gates/mdwidths.sh
	/bin/sh scripts/gates/noblock.sh
	/bin/sh scripts/gates/nocli-shell.sh
	SCAN_MIN=135 /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=53 TYPES='Refresh' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=81 TYPES='Launch' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=111 HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot Attribution' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=26 HOMEFILE=src/launch.rs TYPES='Outcome' /bin/sh scripts/gates/nodefault-ui.sh
	/bin/sh scripts/gates/noio-view.sh
	/bin/sh scripts/gates/nojson-seam.sh
	/bin/sh scripts/gates/nolit-change.sh
	/bin/sh scripts/gates/noraw-grep.sh
	/bin/sh scripts/gates/nosleep.sh
	/bin/sh scripts/gates/nospawn-grep.sh
	/bin/sh scripts/gates/notabseam.sh
	/bin/sh scripts/gates/nowaiver.sh
	/bin/sh scripts/gates/openspec-untouched.sh
	/bin/sh scripts/gates/readonly-ui.sh
	/bin/sh scripts/gates/readseam.sh
	/bin/sh scripts/gates/taskseam.sh
	/bin/sh scripts/gates/taskwidths.sh
	/bin/sh scripts/gates/watchseam.sh
	/bin/sh scripts/gates/widths.sh
	python3 scripts/gates/gate-mech1.py

gates-full:
	DEPS_FULL=1 /bin/sh scripts/gates/deps.sh

check: fmt-check lint gates test coverage
