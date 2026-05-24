SHELL := /bin/bash
DOTNET_SLN := orleans-rust-client.slnx

.PHONY: all check rust-fmt rust-clippy rust-test rust-build dotnet-build dotnet-format e2e clean help

all: check

help:
	@echo "Targets:"
	@echo "  check         Full local suite: fmt, clippy, tests, dotnet build/format, e2e"
	@echo "  rust-build    Build the Rust workspace"
	@echo "  rust-fmt      Check Rust formatting"
	@echo "  rust-clippy   Lint Rust with warnings denied"
	@echo "  rust-test     Run Rust unit/doc tests"
	@echo "  dotnet-build  Build the .NET solution (Release)"
	@echo "  dotnet-format Verify .NET formatting"
	@echo "  e2e           End-to-end: start a silo + bridge, run the Rust client"
	@echo "  clean         Remove build artifacts"

# Full local verification. There is no hosted CI; run this before pushing.
check: rust-fmt rust-clippy rust-test dotnet-format dotnet-build e2e

rust-build:
	cargo build --workspace

rust-fmt:
	cargo fmt --all -- --check

rust-clippy:
	cargo clippy --workspace --all-targets -- -D warnings

rust-test:
	cargo test --workspace

dotnet-build:
	dotnet build $(DOTNET_SLN) -c Release

dotnet-format:
	dotnet format $(DOTNET_SLN) --verify-no-changes

# Requires the .NET SDK and protoc; builds the sample, starts a silo + bridge,
# and runs the Rust client against them.
e2e:
	cargo test -p orleans-bridge-integration --release -- --ignored --nocapture --test-threads=1

clean:
	cargo clean
	dotnet clean $(DOTNET_SLN) -c Release || true
