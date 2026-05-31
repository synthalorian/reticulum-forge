//! Network check modules for `forge test`.
//!
//! Each module implements a check that analyzes a Reticulum network topology
//! (represented as a petgraph graph) for specific properties.

pub mod connectivity;
pub mod latency;
pub mod redundancy;

use serde::Serialize;

/// Category of check.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum CheckCategory {
    Connectivity,
    Latency,
    Redundancy,
    Policy,
}

/// Status of a single check.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warning,
    Error,
}

/// Result of a single check.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub category: CheckCategory,
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub details: Option<String>,
}

impl CheckResult {
    pub fn pass(category: CheckCategory, name: &str, message: &str) -> Self {
        CheckResult {
            category,
            name: name.to_string(),
            status: CheckStatus::Pass,
            message: message.to_string(),
            details: None,
        }
    }

    pub fn fail(category: CheckCategory, name: &str, message: &str) -> Self {
        CheckResult {
            category,
            name: name.to_string(),
            status: CheckStatus::Fail,
            message: message.to_string(),
            details: None,
        }
    }

    pub fn warn(category: CheckCategory, name: &str, message: &str) -> Self {
        CheckResult {
            category,
            name: name.to_string(),
            status: CheckStatus::Warning,
            message: message.to_string(),
            details: None,
        }
    }

    pub fn error(category: CheckCategory, name: &str, message: &str) -> Self {
        CheckResult {
            category,
            name: name.to_string(),
            status: CheckStatus::Error,
            message: message.to_string(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }
}
