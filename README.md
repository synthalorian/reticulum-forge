# ⚒ Reticulum Forge

> CLI toolkit for building, testing, and deploying Reticulum networks — like terraform for mesh

[![CI](https://github.com/synthalorian/reticulum-forge/actions/workflows/ci.yml/badge.svg)](https://github.com/synthalorian/reticulum-forge/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-1.80%2B-orange)
![License](https://img.shields.io/badge/license-Apache%202.0-blue)

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
forge generate --hardware rnode-lora --param freq=868mhz --param bw=125khz --param sf=10

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
│   ├── main.rs              # Entry point
│   ├── cli.rs               # Clap CLI definitions
│   ├── config.rs            # Config loading & validation
│   ├── error.rs             # Error types
│   ├── template.rs          # Tera template engine
│   ├── test_runner.rs       # Test orchestration
│   ├── policy.rs            # Network policy checks
│   ├── commands/            # Subcommand implementations
│   │   ├── init.rs          # forge init
│   │   ├── generate.rs      # forge generate
│   │   ├── simulate.rs      # forge simulate
│   │   ├── deploy.rs        # forge deploy
│   │   ├── test.rs          # forge test
│   │   └── monitor.rs       # forge monitor
│   ├── generate/            # Config generation (stub)
│   ├── hardware/            # Hardware specs & validation
│   ├── simulate/            # Network simulator engine
│   ├── deploy/              # SSH deployment & provisioning
│   ├── monitor/             # TUI dashboard
│   └── checks/              # Graph-based network checks
├── tests/
│   └── cli_tests.rs         # Integration tests (48)
├── justfile                 # Build/release/test automation
├── .github/workflows/       # CI pipeline
├── Cargo.toml
├── PLAN.md                  # Implementation plan & status
└── README.md
```

### Commands

| Command | Status | Description |
|---------|--------|-------------|
| `forge init` | ✅ | Scaffold a new Reticulum network project |
| `forge generate` | ✅ | Generate interface configs for 5 hardware types |
| `forge simulate` | ✅ | Run virtual mesh network simulations (4 topologies) |
| `forge test` | ✅ | Connectivity, latency, redundancy, and policy checks (4 formats) |
| `forge deploy` | ✅ | SSH-based rolling deployment with rollback and provisioning |
| `forge monitor` | ✅ | Real-time TUI dashboard with SSH health polling |

### Tech Stack

| Component     | Technology                    |
|--------------|-------------------------------|
| Language      | Rust 1.80+                    |
| CLI           | Clap 4 (derive macros)        |
| Async Runtime | Tokio                         |
| Serialization | serde (TOML, JSON, YAML)      |
| TUI           | ratatui + crossterm           |
| SSH           | russh                         |
| Templating    | Tera                          |
| Graph         | petgraph                      |
| Testing       | assert_cmd, proptest          |

## Development

```bash
just check    # fmt + clippy + test
just build    # release build
just audit    # security audit
just watch    # auto-run tests on change
```

## License

Apache License 2.0

## Credits

Built by **synth** (synthalorian) with **synthshark**.
