SHELL := /bin/bash
DOTNET_SLN := orleans-rust-client.slnx
COVERAGE_DIR := target/coverage
# Excludes generated proto, build scripts, the CLI shim, and test code.
RUST_COV_IGNORE := (/target/|build\.rs|/tests/|src/main\.rs|generated\.rs)

.PHONY: all check rust-fmt rust-clippy rust-test rust-build dotnet-build dotnet-format dotnet-test e2e \
        coverage coverage-rust coverage-dotnet clean help

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
	@echo "  dotnet-test   Run the .NET unit tests"
	@echo "  e2e           End-to-end: start a silo + bridge, run the Rust client"
	@echo "  coverage      Rust + .NET line coverage (unit + e2e)"
	@echo "  coverage-rust    Rust coverage via cargo-llvm-cov (unit + e2e)"
	@echo "  coverage-dotnet  .NET coverage via dotnet-coverage (unit + e2e)"
	@echo "  clean         Remove build artifacts"

# Full local verification. There is no hosted CI; run this before pushing.
check: rust-fmt rust-clippy rust-test dotnet-format dotnet-build dotnet-test e2e

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

dotnet-test:
	dotnet test $(DOTNET_SLN) -c Release

# Requires the .NET SDK and protoc; builds the sample, starts a silo + bridge,
# and runs the Rust client against them.
e2e:
	cargo test -p orleans-bridge-integration --release -- --ignored --nocapture --test-threads=1

# --- Coverage -----------------------------------------------------------------
# Prerequisites (install once):
#   cargo install cargo-llvm-cov
#   rustup component add llvm-tools-preview
#   dotnet tool install --global dotnet-coverage
# Both targets include the live e2e suite, so the .NET SDK must be available.

coverage: coverage-rust coverage-dotnet

# Rust line coverage over the workspace, including the ignored e2e tests (so the
# client's gRPC paths are counted, since the client runs in the test process).
coverage-rust:
	cargo llvm-cov --workspace --all-features \
		--ignore-filename-regex '$(RUST_COV_IGNORE)' \
		--summary-only -- --include-ignored --test-threads=1

# .NET line coverage merged across the unit tests and the live e2e (which runs
# the bridge/silo as child processes that dotnet-coverage also instruments).
coverage-dotnet:
	@mkdir -p $(COVERAGE_DIR)
	dotnet-coverage collect -f cobertura -o $(COVERAGE_DIR)/dotnet.cobertura.xml -- \
		bash -c 'dotnet test $(DOTNET_SLN) -c Release && \
		cargo test -p orleans-bridge-integration --release -- --ignored --test-threads=1'
	python3 scripts/cobertura_summary.py $(COVERAGE_DIR)/dotnet.cobertura.xml

clean:
	cargo clean
	dotnet clean $(DOTNET_SLN) -c Release || true
	rm -rf $(COVERAGE_DIR)
