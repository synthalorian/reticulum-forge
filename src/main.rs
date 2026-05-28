//! Reticulum Forge — CLI toolkit for building, testing, and deploying Reticulum networks.

mod cli;

fn main() -> anyhow::Result<()> {
    cli::run()
}
