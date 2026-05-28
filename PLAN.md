# Reticulum Forge — Implementation Plan

## Overview

Phased implementation of a Rust CLI toolkit for building, testing, simulating, and deploying Reticulum mesh networks.

## Phase 1: CLI Foundation (Weeks 1–2)

**Goal:** Bootable CLI with all subcommand stubs, config parsing, and project scaffolding.

### Tasks
- [ ] Clap 4 derive-based CLI with subcommands: `init`, `generate`, `simulate`, `deploy`, `test`, `monitor`
- [ ] `forge init <name>` — creates project directory with:
  - `forge.toml` (project config)
  - `nodes/` (node definitions)
  - `interfaces/` (interface templates)
  - `deploy/` (deployment manifests)
- [ ] Config loading with serde: parse `forge.toml`, validate schema
- [ ] Template engine (Tera) for config generation
- [ ] Error handling with `anyhow` / `thiserror`
- [ ] Colored terminal output with `indicatif` spinners
- [ ] `--verbose` / `--quiet` logging with `tracing`

### File Touchpoints
```
src/main.rs
src/cli.rs
src/config.rs
src/error.rs
src/template.rs
src/commands/init.rs
```

## Phase 2: Config Generator (Weeks 3–4)

**Goal:** `forge generate` produces valid Reticulum interface configs for real hardware.

### Tasks
- [ ] Hardware database — validated parameter ranges per device type:
  - RNode LoRa (frequencies, BW, SF, TX power, coding rate)
  - Serial TNC (baud rates, flow control, KISS params)
  - TCP client/server (host, port, keepalive)
  - AutoInterface (group ID, multicast)
- [ ] Template set — Tera templates for each interface type → Reticulum config format
- [ ] `forge generate --hardware <type> [options]` — interactive and CLI-driven modes
- [ ] Config validation — verify generated configs against Reticulum spec
- [ ] Multi-interface node support — generate full node configs with multiple interfaces
- [ ] Output formats: Reticulum native (.config), JSON, YAML

### File Touchpoints
```
src/commands/generate.rs
src/hardware/
src/hardware/rnode.rs
src/hardware/serial.rs
src/hardware/tcp.rs
src/hardware/auto_interface.rs
src/templates/reticulum.config.tera
src/templates/rnode.config.tera
```

## Phase 3: Network Simulator (Weeks 5–8)

**Goal:** `forge simulate` runs a virtual Reticulum network in-process.

### Tasks
- [ ] `VirtualNode` — simulated Reticulum node with identity, interfaces, routing table
- [ ] `VirtualLink` — simulated link between nodes with configurable:
  - Latency (fixed, jitter, distribution)
  - Packet loss (percentage, burst pattern)
  - Bandwidth (bytes/sec limit)
  - Signal quality (SNR simulation)
- [ ] Topology generators:
  - `mesh` — full/partial mesh with configurable connectivity
  - `star` — hub-and-spoke
  - `ring` — circular topology
  - `chain` — linear chain
  - `custom` — from DOT/adjacency file
- [ ] Event-driven simulation engine with Tokio channels
- [ ] Simulated announce propagation — test how announces spread through topology
- [ ] Simulated path discovery — validate routing in complex topologies
- [ ] Metrics collection: delivery rate, latency histogram, hop counts
- [ ] Output results as:
  - Terminal summary table
  - JSON report
  - DOT/Graphviz topology visualization (with link quality coloring)
- [ ] Property-based tests (proptest) for simulator correctness

### File Touchpoints
```
src/simulate/node.rs
src/simulate/link.rs
src/simulate/topology.rs
src/simulate/engine.rs
src/simulate/metrics.rs
src/simulate/report.rs
src/commands/simulate.rs
```

## Phase 4: Config Validation & Testing (Weeks 9–10)

**Goal:** `forge test` validates configs and tests real/simulated networks.

### Tasks
- [ ] Config lint — syntax, semantic, and policy validation
- [ ] Connectivity test — verify all defined node pairs can reach each other
- [ ] Latency test — measure round-trip time between nodes
- [ ] Bandwidth test — estimate throughput over paths
- [ ] Redundancy analysis — identify single points of failure
- [ ] Policy engine — define and check network policies:
  - All links must be encrypted
  - No single-node cut vertices
  - Maximum hop count threshold
- [ ] CI output formats: TAP, JUnit XML, GitHub Actions annotations
- [ ] `forge test --watch` — re-test on config file changes

### File Touchpoints
```
src/commands/test.rs
src/test_runner.rs
src/policy.rs
src/checks/
src/checks/connectivity.rs
src/checks/latency.rs
src/checks/redundancy.rs
```

## Phase 5: Deployment (Weeks 11–13)

**Goal:** `forge deploy` pushes configs to remote nodes via SSH.

### Tasks
- [ ] Inventory management — `nodes.toml` defines fleet:
  - Host, SSH credentials, RNS install path
  - Tags for group deployment (e.g., `[lora-nodes]`, `[backbone]`)
- [ ] SSH deployment via `russh`:
  - Transfer config files to remote nodes
  - Restart rnsd service
  - Verify node comes back online
- [ ] Rolling deployment — one node at a time with health check between
- [ ] Rollback — restore previous config if health check fails
- [ ] Provisioning — `forge deploy --provision` does full setup:
  - Install Python + pip
  - Install RNS
  - Deploy config
  - Enable systemd service
  - Verify connectivity
- [ ] Parallel deployment mode (with concurrency limit)
- [ ] Dry-run mode — show what would be deployed without doing it

### File Touchpoints
```
src/deploy/inventory.rs
src/deploy/ssh.rs
src/deploy/provision.rs
src/deploy/health.rs
src/deploy/rollback.rs
src/commands/deploy.rs
```

## Phase 6: TUI Monitor (Weeks 14–15)

**Goal:** `forge monitor` shows a real-time terminal dashboard.

### Tasks
- [ ] ratatui-based TUI with layout:
  - Top bar: summary stats (total nodes, healthy, degraded, offline)
  - Left panel: node list with status indicators (🟢🟡🔴)
  - Right panel: selected node detail (interfaces, bandwidth, uptime)
  - Bottom: event log with filtering
- [ ] Connect to nodes via SSH (read rnsd status) or Reticulum
- [ ] Keyboard navigation: ↑↓ to select nodes, Enter for detail, / to filter
- [ ] Color scheme: green (healthy), yellow (degraded), red (offline), blue (informational)
- [ ] Refresh interval configurable (default 2s)
- [ ] Graceful degradation over slow SSH connections

### File Touchpoints
```
src/monitor/app.rs
src/monitor/ui.rs
src/monitor/node_status.rs
src/monitor/event_handler.rs
src/commands/monitor.rs
```

## Phase 7: Polish & Release (Week 16)

**Goal:** v1.0.0 release with cross-platform binaries.

### Tasks
- [ ] Comprehensive integration tests
- [ ] Cross-compilation: `x86_64-linux`, `aarch64-linux`, `x86_64-macos`, `x86_64-windows`
- [ ] GitHub Actions CI: test, clippy, fmt, audit
- [ ] GitHub Release with binary assets
- [ ] Man pages and shell completions (generated by Clap)
- [ ] User guide documentation
- [ ] AUR package for Arch Linux
- [ ] Release v1.0.0

## Success Metrics

| Metric | Target |
|--------|--------|
| `forge init` | < 100ms |
| Config generation | < 50ms per interface |
| Simulation (100 nodes) | > 10,000 packets/sec |
| Deploy (10 nodes) | < 60s total |
| TUI refresh | 60fps rendering |
| Binary size | < 10MB (stripped) |
| Memory (idle) | < 20MB |
| Test coverage | > 85% |
