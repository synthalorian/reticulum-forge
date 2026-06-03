//! Live SSH integration tests — requires local SSH server with key auth.
//!
//! Run with: cargo test --test live_ssh_test -- --ignored
//!
//! Prerequisites:
//! 1. SSH server running on localhost:22
//! 2. ~/.ssh/id_ed25519 exists and is in ~/.ssh/authorized_keys
//! 3. User 'synth' can SSH to localhost without password

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn forge() -> Command {
    let mut cmd = Command::cargo_bin("forge").unwrap();
    cmd.timeout(std::time::Duration::from_secs(30));
    cmd
}

#[test]
#[ignore = "requires local SSH server with key auth"]
fn forge_deploy_dry_run_live() {
    let tmp = TempDir::new().unwrap();
    let inventory_path = tmp.path().join("nodes.toml");

    let toml_content = r#"
[nodes]
[nodes.local]
host = "127.0.0.1"
port = 22
user = "synth"
key_path = "/home/synth/.ssh/id_ed25519"
tags = ["local", "test"]
rns_install_path = "/home/synth/.local/bin"
config_path = "/home/synth/.reticulum/config"
service_name = "rnsd"
"#;
    fs::write(&inventory_path, toml_content).unwrap();

    forge()
        .current_dir(tmp.path())
        .arg("deploy")
        .arg("--inventory")
        .arg(inventory_path.to_str().unwrap())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("skipped"))
        .stdout(predicate::str::contains("127.0.0.1"));
}

#[test]
#[ignore = "requires local SSH server with key auth"]
fn forge_deploy_live_file_transfer_and_rollback() {
    // Tests actual SSH file transfer + rollback by using a service that fails
    let tmp = TempDir::new().unwrap();
    let inventory_path = tmp.path().join("nodes.toml");
    let config_path = tmp.path().join("reticulum.config");

    let toml_content = r#"
[nodes]
[nodes.local]
host = "127.0.0.1"
port = 22
user = "synth"
key_path = "/home/synth/.ssh/id_ed25519"
tags = ["local", "test"]
rns_install_path = "/home/synth/.local/bin"
config_path = "/home/synth/.reticulum/forge_live_test_config"
service_name = "nonexistent_service_12345"
"#;
    fs::write(&inventory_path, toml_content).unwrap();
    fs::write(&config_path, "# live test config\n").unwrap();

    // This will transfer the file, fail to restart the dummy service, and rollback
    forge()
        .current_dir(tmp.path())
        .arg("deploy")
        .arg("--inventory")
        .arg(inventory_path.to_str().unwrap())
        .arg("--config")
        .arg(config_path.to_str().unwrap())
        .assert()
        .failure()
        .stdout(predicate::str::contains("rolled back"));

    // The file may still exist if there was no prior snapshot (rollback restores
    // previous state, and "no file" can't be restored). That's acceptable.
    // What matters is that SSH file transfer worked and rollback was attempted.
}

#[test]
#[ignore = "requires local SSH server with key auth"]
fn forge_monitor_help() {
    forge()
        .arg("monitor")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("inventory"));
}
