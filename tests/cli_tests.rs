use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn forge() -> Command {
    Command::cargo_bin("forge").unwrap()
}

#[test]
fn forge_help() {
    forge()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "CLI toolkit for Reticulum mesh networks",
        ));
}

#[test]
fn forge_version() {
    forge()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("forge"));
}

#[test]
fn forge_init_creates_project() {
    let tmp = TempDir::new().unwrap();
    let project_name = "test-mesh";

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg(project_name)
        .assert()
        .success();

    let project_dir = tmp.path().join(project_name);
    assert!(project_dir.exists());
    assert!(project_dir.join("forge.toml").exists());
    assert!(project_dir.join("nodes").is_dir());
    assert!(project_dir.join("interfaces").is_dir());
    assert!(project_dir.join("deploy").is_dir());

    let toml_content = fs::read_to_string(project_dir.join("forge.toml")).unwrap();
    assert!(toml_content.contains("test-mesh"));
    assert!(toml_content.contains("topology"));
}

#[test]
fn forge_init_rejects_bad_name() {
    forge().arg("init").arg("../etc/passwd").assert().failure();
}

#[test]
fn forge_init_rejects_bad_topology() {
    forge()
        .arg("init")
        .arg("ok-name")
        .arg("--topology")
        .arg("hypercube")
        .assert()
        .failure();
}

#[test]
fn forge_init_quiet_mode() {
    let tmp = TempDir::new().unwrap();

    forge()
        .current_dir(tmp.path())
        .arg("--quiet")
        .arg("init")
        .arg("quiet-net")
        .assert()
        .success();

    let project_dir = tmp.path().join("quiet-net");
    assert!(project_dir.exists());
    assert!(project_dir.join("forge.toml").exists());
}

#[test]
fn forge_init_mesh_topology() {
    let tmp = TempDir::new().unwrap();

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("mesh-net")
        .arg("--topology")
        .arg("mesh")
        .assert()
        .success();

    let toml = fs::read_to_string(tmp.path().join("mesh-net").join("forge.toml")).unwrap();
    assert!(toml.contains("mesh"));
}

#[test]
fn forge_init_star_topology() {
    let tmp = TempDir::new().unwrap();

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("star-net")
        .arg("--topology")
        .arg("star")
        .assert()
        .success();

    let toml = fs::read_to_string(tmp.path().join("star-net").join("forge.toml")).unwrap();
    assert!(toml.contains("star"));
}

#[test]
fn forge_generate_rnode_lora_stdout() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("rnode-lora")
        .arg("--param")
        .arg("freq=868mhz")
        .arg("--param")
        .arg("sf=10")
        .assert()
        .success()
        .stdout(predicate::str::contains("RNodeInterface"))
        .stdout(predicate::str::contains("868000000"));
}

#[test]
fn forge_generate_rnode_to_file() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("rnode.config");

    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("rnode-lora")
        .arg("--param")
        .arg("freq=868mhz")
        .arg("--output")
        .arg(output.to_str().unwrap())
        .assert()
        .success();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("RNodeInterface"));
    assert!(content.contains("868000000"));
}

#[test]
fn forge_generate_serial() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("serial")
        .arg("--param")
        .arg("port=ttyAMA0")
        .arg("--param")
        .arg("baud=115200")
        .assert()
        .success()
        .stdout(predicate::str::contains("SerialInterface"));
}

#[test]
fn forge_generate_tcp_client() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("tcp-client")
        .arg("--param")
        .arg("target_host=peer.example.com")
        .assert()
        .success()
        .stdout(predicate::str::contains("TCPClientInterface"))
        .stdout(predicate::str::contains("peer.example.com"));
}

#[test]
fn forge_generate_tcp_server() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("tcp-server")
        .arg("--param")
        .arg("listen_address=0.0.0.0")
        .assert()
        .success()
        .stdout(predicate::str::contains("TCPServerInterface"));
}

#[test]
fn forge_generate_auto_interface() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("auto")
        .assert()
        .success()
        .stdout(predicate::str::contains("AutoInterface"));
}

#[test]
fn forge_generate_json_format() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("rnode-lora")
        .arg("--param")
        .arg("freq=868mhz")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"interface\""));
}

#[test]
fn forge_generate_yaml_format() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("rnode-lora")
        .arg("--param")
        .arg("freq=868mhz")
        .arg("--format")
        .arg("yaml")
        .assert()
        .success()
        .stdout(predicate::str::contains("interface:"));
}

#[test]
fn forge_generate_bad_hardware() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("nonexistent-radio")
        .assert()
        .failure();
}

#[test]
fn forge_generate_bad_format() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("rnode-lora")
        .arg("--param")
        .arg("freq=868mhz")
        .arg("--format")
        .arg("xml")
        .assert()
        .failure();
}

#[test]
fn forge_generate_missing_required_param() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("tcp-client")
        .assert()
        .failure();
}

#[test]
fn forge_generate_invalid_param_value() {
    forge()
        .arg("generate")
        .arg("--hardware")
        .arg("rnode-lora")
        .arg("--param")
        .arg("freq=868mhz")
        .arg("--param")
        .arg("sf=99")
        .assert()
        .failure();
}

#[test]
fn forge_simulate_mesh_table() {
    forge()
        .arg("simulate")
        .arg("--nodes")
        .arg("5")
        .arg("--topology")
        .arg("mesh")
        .arg("--duration")
        .arg("10s")
        .arg("--quality")
        .arg("excellent")
        .assert()
        .success()
        .stdout(predicate::str::contains("Delivery rate"))
        .stdout(predicate::str::contains("Nodes"));
}

#[test]
fn forge_simulate_json() {
    forge()
        .arg("simulate")
        .arg("--nodes")
        .arg("5")
        .arg("--duration")
        .arg("10s")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"node_count\""));
}

#[test]
fn forge_simulate_dot() {
    forge()
        .arg("simulate")
        .arg("--nodes")
        .arg("5")
        .arg("--duration")
        .arg("10s")
        .arg("--format")
        .arg("dot")
        .assert()
        .success()
        .stdout(predicate::str::contains("digraph"));
}

#[test]
fn forge_simulate_to_file() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("sim.json");

    forge()
        .arg("simulate")
        .arg("--nodes")
        .arg("5")
        .arg("--duration")
        .arg("10s")
        .arg("--format")
        .arg("json")
        .arg("--output")
        .arg(output.to_str().unwrap())
        .assert()
        .success();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("node_count"));
}

#[test]
fn forge_simulate_min_nodes() {
    forge()
        .arg("simulate")
        .arg("--nodes")
        .arg("2")
        .arg("--duration")
        .arg("5s")
        .assert()
        .success();
}

#[test]
fn forge_simulate_bad_topology() {
    forge()
        .arg("simulate")
        .arg("--topology")
        .arg("hypercube")
        .assert()
        .failure();
}

#[test]
fn forge_simulate_bad_quality() {
    forge()
        .arg("simulate")
        .arg("--quality")
        .arg("terrible")
        .assert()
        .failure();
}

#[test]
fn forge_simulate_chain_topology() {
    forge()
        .arg("simulate")
        .arg("--nodes")
        .arg("5")
        .arg("--topology")
        .arg("chain")
        .arg("--duration")
        .arg("10s")
        .assert()
        .success()
        .stdout(predicate::str::contains("Nodes"));
}

#[test]
fn forge_simulate_star_topology() {
    forge()
        .arg("simulate")
        .arg("--nodes")
        .arg("5")
        .arg("--topology")
        .arg("star")
        .arg("--duration")
        .arg("10s")
        .assert()
        .success();
}

#[test]
fn forge_test_connectivity() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("test-net");

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("test-net")
        .assert()
        .success();

    forge()
        .current_dir(&project_dir)
        .arg("test")
        .arg("--check")
        .arg("connectivity")
        .assert()
        .success()
        .stdout(predicate::str::contains("passed"));
}

#[test]
fn forge_test_all_checks() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("test-net");

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("test-net")
        .assert()
        .success();

    forge()
        .current_dir(&project_dir)
        .arg("test")
        .assert()
        .success()
        .stdout(predicate::str::contains("passed"));
}

#[test]
fn forge_test_json_output() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("test-net");

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("test-net")
        .assert()
        .success();

    forge()
        .current_dir(&project_dir)
        .arg("test")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"passed\""));
}

#[test]
fn forge_test_tap_output() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("test-net");

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("test-net")
        .assert()
        .success();

    forge()
        .current_dir(&project_dir)
        .arg("test")
        .arg("--format")
        .arg("tap")
        .assert()
        .success()
        .stdout(predicate::str::contains("ok"));
}

#[test]
fn forge_test_junit_output() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("test-net");

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("test-net")
        .assert()
        .success();

    forge()
        .current_dir(&project_dir)
        .arg("test")
        .arg("--format")
        .arg("junit")
        .assert()
        .success()
        .stdout(predicate::str::contains("<testsuite"));
}

#[test]
fn forge_test_latency_check() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("test-net");

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("test-net")
        .assert()
        .success();

    forge()
        .current_dir(&project_dir)
        .arg("test")
        .arg("--check")
        .arg("latency")
        .assert()
        .success();
}

#[test]
fn forge_test_redundancy_check() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("test-net");

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("test-net")
        .assert()
        .success();

    forge()
        .current_dir(&project_dir)
        .arg("test")
        .arg("--check")
        .arg("redundancy")
        .assert()
        .success();
}

#[test]
fn forge_test_policies_check() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("test-net");

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("test-net")
        .assert()
        .success();

    forge()
        .current_dir(&project_dir)
        .arg("test")
        .arg("--check")
        .arg("policies")
        .assert()
        .success();
}

#[test]
fn forge_test_bad_format() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("test-net");

    forge()
        .current_dir(tmp.path())
        .arg("init")
        .arg("test-net")
        .assert()
        .success();

    forge()
        .current_dir(&project_dir)
        .arg("test")
        .arg("--format")
        .arg("xml")
        .assert()
        .failure();
}

#[test]
fn forge_deploy_dry_run_table() {
    let tmp = TempDir::new().unwrap();
    let inventory_path = tmp.path().join("nodes.toml");

    let toml_content = r#"
[nodes]
[nodes.gateway]
host = "10.0.0.1"
port = 22
user = "root"
key_path = "/dev/null"
tags = ["lora"]
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
        .stdout(predicate::str::contains("10.0.0.1"));
}

#[test]
fn forge_deploy_dry_run_json() {
    let tmp = TempDir::new().unwrap();
    let inventory_path = tmp.path().join("nodes.toml");

    let toml_content = r#"
[nodes]
[nodes.gateway]
host = "10.0.0.1"
port = 22
user = "root"
key_path = "/dev/null"
tags = ["lora"]
"#;
    fs::write(&inventory_path, toml_content).unwrap();

    forge()
        .current_dir(tmp.path())
        .arg("deploy")
        .arg("--inventory")
        .arg(inventory_path.to_str().unwrap())
        .arg("--dry-run")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("gateway"));
}

#[test]
fn forge_deploy_concurrency_zero() {
    forge()
        .arg("deploy")
        .arg("--concurrency")
        .arg("0")
        .assert()
        .failure();
}

#[test]
fn forge_deploy_concurrency_too_high() {
    forge()
        .arg("deploy")
        .arg("--concurrency")
        .arg("100")
        .assert()
        .failure();
}

#[test]
fn forge_deploy_nonexistent_inventory() {
    forge()
        .arg("deploy")
        .arg("--inventory")
        .arg("/nonexistent/path/nodes.toml")
        .assert()
        .failure();
}

#[test]
fn forge_deploy_bad_inventory_path() {
    forge()
        .arg("deploy")
        .arg("--inventory")
        .arg("../../etc/passwd")
        .assert()
        .failure();
}

#[test]
fn forge_bad_subcommand() {
    forge().arg("nonexistent-command").assert().failure();
}
