# Changelog

## v1.0.0 — Reticulum Forge

> CLI toolkit for building, testing, simulating, and deploying Reticulum mesh networks.

### Features

- **`forge init`** — Scaffold a new Reticulum network project with configurable topology (mesh, star, chain, tree).
- **`forge generate`** — Generate interface configs for 5 hardware types: RNode LoRa, TCP Server, UDP Interface, Serial KISS, and I2P.
- **`forge simulate`** — Run virtual mesh network simulations across 4 topologies with configurable node count and duration. Output in table, JSON, dot (GraphViz), or stdout formats.
- **`forge test`** — Execute connectivity, latency, redundancy, and policy checks. Output in TAP, JUnit XML, JSON, or table formats.
- **`forge deploy`** — SSH-based rolling deployment with automatic rollback on failure, remote provisioning, and inventory management.
- **`forge monitor`** — Real-time TUI dashboard with SSH health polling, node status, and network overview.
- **`forge completions`** — Auto-generated shell completions for bash, zsh, fish, and PowerShell via `clap_complete`.
- **`forge man`** — Auto-generated man page via `clap_mangen`.

### Architecture

- **Language:** Rust 1.80+
- **Lines of code:** ~3,500
- **Modules:** 14 source modules + 6 command modules
- **Integration tests:** 45
- **Dependencies:** 25 production, 4 dev

### Build & Distribution

- Cross-compilation support: `aarch64-unknown-linux-gnu` (ARM64/Raspberry Pi), `x86_64-unknown-linux-musl` (static binary)
- GitHub Actions CI: test, clippy, format, audit, cross-build
- GitHub Actions Release: 6-target matrix build with checksums and auto-release
- AUR PKGBUILD with shell completions and man page

### Technical Highlights

- Graph-based network validation using `petgraph`
- Embedded Tera templates (no filesystem loading — prevents template injection)
- SSH key-only auth via `russh` (no password auth)
- TUI dashboard with `ratatui` + `crossterm`
- Proptest-based property testing for simulation invariants

---

Built by **synth** (synthalorian) with **synthshark**.
