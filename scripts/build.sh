#!/bin/sh
# Build the release binary. POSIX sh: Herdr may run this without a login
# shell, so ~/.cargo/bin is not guaranteed to be on PATH even when Rust is
# installed. See SPEC.md -> Build and distribution.
set -eu

if ! command -v cargo >/dev/null 2>&1; then
	if [ -r "$HOME/.cargo/env" ]; then
		echo "cargo not found on PATH; sourcing $HOME/.cargo/env" 1>&2
		. "$HOME/.cargo/env"
	fi
fi

if ! command -v cargo >/dev/null 2>&1; then
	echo "error: cargo not found. Install Rust from https://rustup.rs and retry." 1>&2
	exit 1
fi

cargo build --release
