use crate::checks::connectivity;
use crate::checks::latency;
use crate::checks::{CheckResult, CheckStatus};
use crate::config::ForgeConfig;
use crate::error::{ForgeError, ForgeResult};
use crate::policy::{self, config_to_edges};
use crate::simulate::topology::TopologyType;
use serde::Serialize;
use std::time::Duration;

const MAX_CHECK_NODES: usize = 50;
const DEFAULT_CHECK_NODES: usize = 10;

#[derive(Debug, Serialize)]
pub struct TestReport {
    pub summary: TestSummary,
    pub checks: Vec<CheckResult>,
}

#[derive(Debug, Serialize)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub errors: usize,
}

impl TestReport {
    fn from_results(checks: Vec<CheckResult>) -> Self {
        let total = checks.len();
        let passed = checks
            .iter()
            .filter(|r| r.status == CheckStatus::Pass)
            .count();
        let failed = checks
            .iter()
            .filter(|r| r.status == CheckStatus::Fail)
            .count();
        let warnings = checks
            .iter()
            .filter(|r| r.status == CheckStatus::Warning)
            .count();
        let errors = checks
            .iter()
            .filter(|r| r.status == CheckStatus::Error)
            .count();
        TestReport {
            summary: TestSummary {
                total,
                passed,
                failed,
                warnings,
                errors,
            },
            checks,
        }
    }
}

pub fn run_checks(
    check_type: &str,
    config: &ForgeConfig,
    threshold: Option<Duration>,
    max_hops: Option<u32>,
) -> ForgeResult<TestReport> {
    let mut all_results: Vec<CheckResult> = Vec::new();
    let edges: Vec<(String, String)> = config_to_edges(config);
    let edge_refs: Vec<(&str, &str)> = edges
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();

    match check_type {
        "connectivity" => {
            all_results.extend(connectivity::check_connectivity(&edge_refs));
        }
        "latency" => {
            let topo_type = topo_type_from_config(config);
            all_results.extend(latency::check_latency(
                DEFAULT_CHECK_NODES.min(MAX_CHECK_NODES),
                topo_type,
                "good",
                threshold,
            ));
        }
        "redundancy" => {
            all_results.extend(policy::check_cut_vertices(&edge_refs));
        }
        "policies" => {
            all_results.extend(policy::check_policies(config, &edge_refs, max_hops));
        }
        "all" => {
            all_results.extend(connectivity::check_connectivity(&edge_refs));
            let topo_type = topo_type_from_config(config);
            all_results.extend(latency::check_latency(
                DEFAULT_CHECK_NODES.min(MAX_CHECK_NODES),
                topo_type,
                "good",
                threshold,
            ));
            all_results.extend(policy::check_cut_vertices(&edge_refs));
            all_results.extend(policy::check_policies(config, &edge_refs, max_hops));
        }
        _ => unreachable!(),
    }

    Ok(TestReport::from_results(all_results))
}

pub fn format_table(report: &TestReport) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    writeln!(
        output,
        "{} passed, {} failed, {} warnings, {} errors ({} total checks)",
        report.summary.passed,
        report.summary.failed,
        report.summary.warnings,
        report.summary.errors,
        report.summary.total
    )
    .unwrap();

    use std::collections::BTreeMap;
    let mut by_cat: BTreeMap<String, (usize, usize, usize, usize)> = BTreeMap::new();
    for check in &report.checks {
        let cat = format!("{:?}", check.category);
        let entry = by_cat.entry(cat).or_insert((0, 0, 0, 0));
        entry.0 += 1;
        match check.status {
            CheckStatus::Pass => entry.1 += 1,
            CheckStatus::Fail => entry.2 += 1,
            CheckStatus::Warning => entry.3 += 1,
            CheckStatus::Error => {}
        }
    }

    for (cat, (total, pass, fail, warn)) in &by_cat {
        writeln!(
            output,
            "  {}: {}/{} passed ({} failed, {} warnings)",
            cat, pass, total, fail, warn
        )
        .unwrap();
    }

    if !report.checks.is_empty() {
        writeln!(output, "\n---").unwrap();
    }

    for check in &report.checks {
        let icon = match check.status {
            CheckStatus::Pass => "✓",
            CheckStatus::Fail => "✗",
            CheckStatus::Warning => "⚠",
            CheckStatus::Error => "!",
        };
        writeln!(output, "{} [{}] {}", icon, check.name, check.message).unwrap();
        if let Some(ref details) = check.details {
            writeln!(output, "    {}", details).unwrap();
        }
    }

    output
}

pub fn format_json(report: &TestReport) -> ForgeResult<String> {
    serde_json::to_string_pretty(report).map_err(ForgeError::SerdeJson)
}

pub fn format_tap(report: &TestReport) -> String {
    use std::fmt::Write;
    let mut output = String::new();
    writeln!(output, "1..{}", report.checks.len()).unwrap();
    for (i, check) in report.checks.iter().enumerate() {
        let ok = match check.status {
            CheckStatus::Pass => "ok",
            _ => "not ok",
        };
        writeln!(output, "{} {} - {}", ok, i + 1, check.name).unwrap();
        if let Some(ref details) = check.details {
            writeln!(output, "# {}", details).unwrap();
        }
    }
    output
}

pub fn format_junit(report: &TestReport) -> ForgeResult<String> {
    let mut output = String::new();
    use std::fmt::Write;

    writeln!(output, r#"<?xml version="1.0" encoding="UTF-8"?>"#).unwrap();
    writeln!(
        output,
        r#"<testsuite name="forge.test" tests="{}" failures="{}" errors="{}">"#,
        report.summary.total, report.summary.failed, report.summary.errors
    )
    .unwrap();

    for check in &report.checks {
        let cat = format!("{:?}", check.category).to_lowercase();
        if check.status == CheckStatus::Pass {
            writeln!(
                output,
                r#"  <testcase classname="forge.{}" name="{}" />"#,
                cat, check.name
            )
            .unwrap();
        } else {
            let status_type = match check.status {
                CheckStatus::Fail => r#" type="failure""#,
                CheckStatus::Error => r#" type="error""#,
                _ => "",
            };
            writeln!(
                output,
                r#"  <testcase classname="forge.{}" name="{}">"#,
                cat, check.name
            )
            .unwrap();
            writeln!(
                output,
                r#"    <failure{} message="{}" />"#,
                status_type,
                xml_escape(&check.message)
            )
            .unwrap();
            if let Some(ref details) = check.details {
                writeln!(
                    output,
                    r#"    <system-out>{}</system-out>"#,
                    xml_escape(details)
                )
                .unwrap();
            }
            writeln!(output, r#"  </testcase>"#).unwrap();
        }
    }

    writeln!(output, r#"</testsuite>"#).unwrap();
    Ok(output)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn topo_type_from_config(config: &ForgeConfig) -> TopologyType {
    config
        .project
        .topology
        .as_deref()
        .and_then(TopologyType::from_str)
        .unwrap_or(TopologyType::Mesh)
}
