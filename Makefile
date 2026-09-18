.PHONY: build fmt fmt-check lint test coverage gates gates-full covers-check check

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
	cargo llvm-cov --fail-under-lines 80 --json --output-path target/llvm-cov.json
	cargo llvm-cov report --summary-only
	python3 scripts/coverage-prod.py target/llvm-cov.json tests/degraded-coverage.toml

gates:
	/bin/sh scripts/gates/deps.sh
	env -u GRAPH_WRITE /bin/sh scripts/gates/build-graph.sh
	/bin/sh scripts/gates/wired.sh
	/bin/sh scripts/gates/agentseam.sh
	/bin/sh scripts/gates/launchseam.sh
	LAUNCH=src/open.rs ENTRY='pub fn run_from_env\(' /bin/sh scripts/gates/launchseam.sh
	/bin/sh scripts/gates/colwidth.sh
	/bin/sh scripts/gates/detailwidths.sh
	/bin/sh scripts/gates/helpwidths.sh
	/bin/sh scripts/gates/settingswidths.sh
	/bin/sh scripts/gates/listwidths.sh
	/bin/sh scripts/gates/mdseam.sh
	/bin/sh scripts/gates/mdwidths.sh
	/bin/sh scripts/gates/noblock.sh
	/bin/sh scripts/gates/nocli-shell.sh
	SCAN_MIN=367 TYPES='Dashboard Filter Detail Sections Overlay Selection Edit' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=53 TYPES='Refresh' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=99 TYPES='Launch' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=111 HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot Attribution' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=43 HOMEFILE=src/launch.rs TYPES='Outcome' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=25 TYPES='ArtifactSection' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=55 HOMEFILE=src/ui/help.rs TYPES='Binding Group' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=4 HOMEFILE=src/launch.rs TYPES='Settings' /bin/sh scripts/gates/nodefault-ui.sh
	SCAN_MIN=105 HOMEFILE=src/settings.rs TYPES='Setting KindResolution PanelState' /bin/sh scripts/gates/nodefault-ui.sh
	/bin/sh scripts/gates/noio-view.sh
	/bin/sh scripts/gates/nojson-seam.sh
	/bin/sh scripts/gates/nolit-change.sh
	/bin/sh scripts/gates/noraw-grep.sh
	/bin/sh scripts/gates/nosleep.sh
	/bin/sh scripts/gates/nospawn-grep.sh
	/bin/sh scripts/gates/notabseam.sh
	/bin/sh scripts/gates/nowaiver.sh
	/bin/sh scripts/gates/openspec-untouched.sh
	/bin/sh scripts/gates/palette.sh
	/bin/sh scripts/gates/readonly-ui.sh
	/bin/sh scripts/gates/readseam.sh
	/bin/sh scripts/gates/taskseam.sh
	/bin/sh scripts/gates/taskwidths.sh
	/bin/sh scripts/gates/watchseam.sh
	/bin/sh scripts/gates/widths.sh
	python3 scripts/gates/gate-mech1.py

gates-full:
	DEPS_FULL=1 /bin/sh scripts/gates/deps.sh

# The structural half of the `covers` contract: every range in
# tests/degraded-coverage.toml resolves, is in bounds, and holds a statement. It reads no
# coverage report, so it is the one member of the coverage tier that does not need a green
# suite - which is why `check` runs it before `test` rather than after. Line percentages and
# hotness stay in `coverage`, where a completed run exists to measure them.
covers-check:
	cargo test --all-features --test degraded_coverage

check: fmt-check lint gates covers-check test coverage
