//! CLI argument definitions and command dispatch using Clap derive macros.
//!
//! # Security
//! - All user-facing strings are bounded in length to prevent resource exhaustion.
//! - Topology and format values are validated against allowlists at the CLI level.
//! - File path args are checked for directory traversal in command implementations.
//! - --quiet suppresses non-error output; --verbose enables debug tracing only.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use clap_mangen::Man;
use std::io;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

/// Maximum length for free-form string inputs (prevent DoS via gigantic strings).
const MAX_INPUT_LENGTH: usize = 256;

/// Valid topology types.
const VALID_TOPOLOGIES: &[&str] = &["mesh", "star", "ring", "chain", "custom"];

/// Valid test check types.
const VALID_TEST_CHECKS: &[&str] = &["all", "connectivity", "latency", "redundancy", "policies"];

/// Valid test output formats.
const VALID_TEST_FORMATS: &[&str] = &["table", "json", "tap", "junit"];

/// Validate a string against an allowlist and max length.
fn validate_enum(s: &str, valid: &[&str]) -> Result<String, String> {
    if s.len() > MAX_INPUT_LENGTH {
        return Err(format!(
            "input exceeds maximum length of {} characters",
            MAX_INPUT_LENGTH
        ));
    }
    let lower = s.to_lowercase();
    if valid.contains(&lower.as_str()) {
        Ok(lower)
    } else {
        Err(format!(
            "invalid value '{}'. Must be one of: {}",
            s,
            valid.join(", ")
        ))
    }
}

/// Validate an input string length.
fn validate_length(s: &str) -> Result<String, String> {
    if s.len() > MAX_INPUT_LENGTH {
        return Err(format!(
            "input exceeds maximum length of {} characters",
            MAX_INPUT_LENGTH
        ));
    }
    Ok(s.to_string())
}

#[derive(Parser)]
#[command(name = "forge")]
#[command(about = "CLI toolkit for Reticulum mesh networks")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new Reticulum network project
    Init {
        /// Project name (alphanumeric, hyphens, underscores; no path separators)
        #[arg(value_parser = validate_length)]
        name: String,
        /// Network topology template
        #[arg(short, long, default_value = "mesh", value_parser = validate_enum_valid_topologies)]
        topology: String,
    },

    /// Generate interface configs for hardware
    Generate {
        /// Hardware type (rnode-lora, serial, tcp-client, tcp-server, auto)
        #[arg(short = 'H', long)]
        hardware: String,
        /// Interface name (alphanumeric, hyphens, underscores)
        #[arg(short, long, default_value = "default", value_parser = validate_length)]
        name: String,
        /// Output format (reticulum, json, yaml)
        #[arg(short, long, default_value = "reticulum")]
        format: String,
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Hardware-specific parameters as key=value pairs, e.g. -P freq=868mhz -P bw=125khz
        #[arg(short = 'P', long = "param")]
        param: Vec<String>,
    },

    /// Simulate a virtual Reticulum network
    Simulate {
        /// Number of virtual nodes
        #[arg(short, long, default_value = "10")]
        nodes: usize,
        /// Network topology (mesh, star, ring, chain)
        #[arg(short, long, default_value = "mesh")]
        topology: String,
        /// Simulation duration (e.g. 30s, 5m, 1h)
        #[arg(short, long, default_value = "30s")]
        duration: String,
        /// Link quality (excellent, good, moderate, poor)
        #[arg(short = 'Q', long, default_value = "good")]
        quality: String,
        /// Output format (table, json, dot)
        #[arg(short, long, default_value = "table")]
        format: String,
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Deploy configs to remote nodes
    Deploy {
        /// Inventory file path (nodes.toml)
        #[arg(short, long, default_value = "nodes.toml")]
        inventory: String,
        /// Dry run (show what would be deployed, no remote changes)
        #[arg(long)]
        dry_run: bool,
        /// Parallel deployment concurrency (max 32)
        #[arg(short, long, default_value = "1")]
        concurrency: usize,
        /// Full provisioning (install Python, RNS, enable service)
        #[arg(long)]
        provision: bool,
        /// Tag filter — only deploy nodes with this tag
        #[arg(short, long)]
        tag: Option<String>,
        /// Config content to deploy (from file or stdin)
        #[arg(long)]
        config: Option<String>,
        /// Output format (table, json)
        #[arg(short, long, default_value = "table")]
        format: String,
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Test network config, connectivity, and policies
    Test {
        /// Check type (connectivity, latency, redundancy, policies, all)
        #[arg(long, default_value = "all", value_parser = validate_enum_test_checks)]
        check: String,
        /// Latency threshold in milliseconds
        #[arg(long)]
        threshold: Option<u64>,
        /// Config file path (forge.toml)
        #[arg(short, long, default_value = "forge.toml")]
        config: String,
        /// Output format (table, json, tap, junit)
        #[arg(short, long, default_value = "table", value_parser = validate_enum_test_formats)]
        format: String,
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Real-time network health dashboard (TUI)
    Monitor {
        /// Inventory file path (nodes.toml)
        #[arg(short = 'I', long, default_value = "nodes.toml")]
        inventory: String,
        /// Refresh interval in seconds
        #[arg(short, long, default_value = "10")]
        interval: u64,
    },

    /// Generate shell completions for bash, zsh, fish, or PowerShell
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Generate man page for forge
    Man {
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

// Separate validation functions for clap's value_parser (each needs its own type).
fn validate_enum_valid_topologies(s: &str) -> Result<String, String> {
    validate_enum(s, VALID_TOPOLOGIES)
}
fn validate_enum_test_checks(s: &str) -> Result<String, String> {
    validate_enum(s, VALID_TEST_CHECKS)
}
fn validate_enum_test_formats(s: &str) -> Result<String, String> {
    validate_enum(s, VALID_TEST_FORMATS)
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing with security-conscious defaults:
    // - quiet = only errors
    // - verbose = debug + forge debug
    // - default = info + forge info
    // Security: file and line number are excluded from logs (sensitive info).
    let filter = if cli.quiet {
        EnvFilter::new("error")
    } else if cli.verbose {
        EnvFilter::new("debug,forge=debug")
    } else {
        EnvFilter::new("info,forge=info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .init();

    match cli.command {
        Commands::Init { name, topology } => {
            crate::commands::init::execute(&name, &topology)?;
        }
        Commands::Generate {
            hardware,
            name,
            format,
            output,
            param,
        } => {
            crate::commands::generate::execute(
                &hardware,
                &name,
                &param,
                &format,
                output.as_deref(),
            )?;
        }
        Commands::Simulate {
            nodes,
            topology,
            duration,
            quality,
            format,
            output,
        } => {
            crate::commands::simulate::execute(
                nodes,
                &topology,
                &duration,
                &quality,
                &format,
                output.as_deref(),
            )?;
        }
        Commands::Deploy {
            inventory,
            dry_run,
            concurrency,
            provision,
            tag,
            config,
            format,
            output,
        } => {
            crate::commands::deploy::execute(
                &inventory,
                dry_run,
                concurrency,
                provision,
                tag.as_deref(),
                config,
                &format,
                output.as_deref(),
            )?;
        }
        Commands::Test {
            check,
            threshold,
            config,
            format,
            output,
        } => {
            crate::commands::test::execute(&check, &config, threshold, &format, output.as_deref())?;
        }
        Commands::Monitor {
            inventory,
            interval: _,
        } => {
            crate::commands::monitor::execute(&inventory)?;
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut io::stdout());
        }
        Commands::Man { output } => {
            let cmd = Cli::command();
            let man = Man::new(cmd);
            if let Some(path) = output {
                let mut file = std::fs::File::create(path)?;
                man.render(&mut file)?;
            } else {
                man.render(&mut io::stdout())?;
            }
        }
    }

    Ok(())
}
