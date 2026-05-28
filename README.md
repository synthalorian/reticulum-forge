# ⚒ Reticulum Forge

> CLI toolkit for building, testing, and deploying Reticulum networks — like terraform for mesh

```
    ╔═════════════════════════════════════════════════════════════════════╗
    ║                    R E T I C U L U M   F O R G E                   ║
    ║                                                                     ║
    ║   $ forge init my-network                                           ║
    ║   $ forge generate --hardware rnode-lora --freq 868mhz              ║
    ║   $ forge simulate --nodes 10 --topology mesh                      ║
    ║   $ forge deploy --target pi@10.0.1.50                             ║
    ║   $ forge monitor                                                   ║
    ║   $ forge test --check connectivity --check latency                ║
    ║                                                                     ║
    ║   ┌─────────────────────────────────────────────────────────────┐  ║
    ║   │                     Forge CLI                               │  ║
    ║   │                                                             │  ║
    ║   │  ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌──────────────┐ │  ║
    ║   │  │   init   │ │ generate  │ │ simulate │ │    deploy    │ │  ║
    ║   │  │ scaffold │ │   config  │ │  virtual │ │    push &    │ │  ║
    ║   │  │ project  │ │ per-hw    │ │  network │ │   provision  │ │  ║
    ║   │  └──────────┘ └───────────┘ └──────────┘ └──────────────┘ │  ║
    ║   │                                                             │  ║
    ║   │  ┌──────────┐ ┌───────────┐ ┌──────────┐                  │  ║
    ║   │  │  monitor │ │   test    │ │ validate │                  │  ║
    ║   │  │   TUI    │ │  network  │ │  configs │                  │  ║
    ║   │  │dashboard │ │  health   │ │  & specs │                  │  ║
    ║   │  └──────────┘ └───────────┘ └──────────┘                  │  ║
    ║   │                                                             │  ║
    ║   │  ┌───────────────────────────────────────────────────────┐  │  ║
    ║   │  │              Config Layer (TOML + Templates)          │  │  ║
    ║   │  └───────────────────────────────────────────────────────┘  │  ║
    ║   └─────────────────────────────────────────────────────────────┘  ║
    ╚═════════════════════════════════════════════════════════════════════╝
```

## Overview

Reticulum Forge is a CLI toolkit for building, testing, simulating, and deploying [Reticulum](https://reticulum.network/) mesh networks. Think of it as **terraform for mesh networks** — define your desired network topology in config, simulate it locally, validate it, then deploy to physical nodes.

Built in **Rust** for performance, reliability, and cross-platform support (Linux, macOS, Windows, ARM).

## Features

### 🏗️ `forge init` — Project Scaffolding
- Bootstrap a new Reticulum network project with directory structure
- Generate default config templates for common topologies (star, mesh, chain)
- Create node definitions, interface configs, and deployment manifests

### ⚡ `forge generate` — Config Generation
- Generate Reticulum interface configs for specific hardware:
  - **RNode** (LoRa) — frequency, bandwidth, TX power, spreading factor
  - **Serial** (TNC, KISS) — baud rate, port, flow control
  - **TCP** — host, port, peering credentials
  - **AutoInterface** — group ID, multicast settings
- Output in Reticulum's native config format or JSON/YAML
- Hardware database with validated parameter ranges

### 🔬 `forge simulate` — Network Simulator
- Spin up virtual Reticulum nodes in-process (no hardware needed)
- Define topology: mesh, star, ring, custom graph
- Simulate link quality, latency, packet loss
- Test announce propagation, path discovery, and LXMF routing
- Visualize simulation results as DOT/Graphviz graphs
- Fast — thousands of virtual packets per second

### 🚀 `forge deploy` — Deployment Automation
- Push validated configs to remote nodes via SSH or Reticulum
- Rolling deployments with health checks
- Rollback on failure
- Provision new nodes from scratch (install RNS, configure systemd, deploy config)
- Inventory management — track your fleet of nodes

### ✅ `forge test` — Network Validation
- Verify config syntax and semantic correctness
- Test connectivity between node pairs
- Measure latency, throughput, and packet loss
- Validate against network policies (encryption requirements, allowed interfaces)
- CI-friendly output (TAP, JUnit XML)

### 📊 `forge monitor` — Real-Time TUI Dashboard
- Terminal UI showing live network health
- Node status grid with color-coded indicators
- Bandwidth graphs, link quality heatmaps
- Event log with filtering
- Runs over SSH for remote monitoring

## Tech Stack

| Component     | Technology                    |
|--------------|-------------------------------|
| Language      | Rust 1.80+                    |
| CLI           | Clap 4 (derive macros)        |
| Async Runtime | Tokio                         |
| Serialization | serde (TOML, JSON, YAML)      |
| TUI           | ratatui                       |
| SSH           | russh                         |
| Templating    | Tera                          |
| Graphviz      | petgraph                      |
| Testing       | assert_cmd, proptest          |

## Quick Start

### Install

```bash
# From source
git clone https://github.com/synthalorian/reticulum-forge.git
cd reticulum-forge
cargo install --path .

# Or download a release binary
curl -sSL https://github.com/synthalorian/reticulum-forge/releases/latest/download/forge-linux-amd64 -o forge
chmod +x forge
```

### Usage

```bash
# Create a new network project
forge init my-mesh-network
cd my-mesh-network

# Generate interface configs for LoRa nodes
forge generate --hardware rnode-lora --freq 868mhz --bw 125khz --sf 10

# Simulate a 20-node mesh network
forge simulate --nodes 20 --topology mesh --duration 60s

# Test connectivity between all node pairs
forge test --check connectivity --check latency --threshold 500ms

# Deploy to your fleet
forge deploy --inventory nodes.toml

# Monitor in real-time
forge monitor
```

## Project Structure

```
reticulum-forge/
├── src/
│   ├── main.rs            # Entry point
│   ├── cli.rs             # Clap CLI definitions
│   ├── config.rs          # Config loading & validation
│   ├── simulate/          # Network simulator engine
│   ├── deploy/            # SSH/Reticulum deployment
│   ├── monitor/           # TUI dashboard
│   └── generate/          # Config generation templates
├── tests/                 # Integration tests
├── Cargo.toml
└── README.md
```

## License

Apache License 2.0 — see [LICENSE](LICENSE).

## Credits

Created by **synthalorian** (Carter) with assistance from **synthclaw**.
