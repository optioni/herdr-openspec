.PHONY: build fmt fmt-check lint test coverage check

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

check: fmt-check lint test coverage
