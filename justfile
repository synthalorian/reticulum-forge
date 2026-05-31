_default:
    @just --list

# Run all checks: format, clippy, tests
check: fmt clippy test

# Format code
fmt:
    cargo fmt --all -- --check

# Run clippy
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run tests
test:
    cargo test --all-features

# Run tests with output shown
test-verbose:
    cargo test --all-features -- --nocapture

# Build release binary
build:
    cargo build --release

# Build optimized release binary
build-release:
    RUSTFLAGS="-C target-cpu=native" cargo build --release

# Generate shell completions
completions:
    @mkdir -p completions
    @cargo run -- completions bash > completions/forge.bash
    @cargo run -- completions zsh > completions/_forge
    @cargo run -- completions fish > completions/forge.fish
    @echo "Completions written to completions/"

# Strip and compress the release binary
release: build
    strip target/release/forge
    upx --best --lzma target/release/forge || true
    @ls -lh target/release/forge

# Watch for changes and re-run tests
watch:
    cargo watch -x test

# Clean build artifacts
clean:
    cargo clean

# Security audit dependencies
audit:
    cargo audit

# Check for outdated dependencies
outdated:
    cargo outdated || cargo update --dry-run

# Install the binary locally
install:
    cargo install --path . --force

# Cross-compile for ARM64 (Raspberry Pi, etc.)
cross-arm64:
    cross build --target aarch64-unknown-linux-gnu --release

# Cross-compile statically linked
cross-musl:
    cross build --target x86_64-unknown-linux-musl --release

# Generate docs
doc:
    cargo doc --no-deps --open