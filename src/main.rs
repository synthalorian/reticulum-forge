//! Reticulum Forge — CLI toolkit for building, testing, and deploying Reticulum networks.

mod checks;
mod cli;
mod commands;
mod config;
mod deploy;
mod error;
mod hardware;
mod monitor;
mod policy;
mod simulate;
mod template;
mod test_runner;

fn main() -> anyhow::Result<()> {
    // Tracing is initialized in cli::run() after parsing verbosity flags
    cli::run()
}
