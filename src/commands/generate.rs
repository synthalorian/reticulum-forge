//! `forge generate` — produce validated Reticulum interface configs.
//!
//! # Security
//! - All user-supplied parameters are validated against strict hardware schemas.
//! - Path/directory traversal is blocked in port and path parameters.
//! - Shell metacharacters are rejected in string parameters.
//! - Template injection is prevented by Tera's auto-escaping and embedded-only templates.
//! - File writes use restrictive permissions (0o644).
//! - Output format is validated against a whitelist (reticulum, json, yaml).

use crate::error::{ForgeError, ForgeResult};
use crate::hardware::{self, parse_and_validate, Param};
use crate::template::TemplateEngine;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// Run the generate command.
///
/// * `hardware_name` — CLI string identifying the hardware type (e.g. "rnode-lora").
/// * `interface_name` — user-chosen name for this interface (sanitized).
/// * `params` — raw `key=value` pairs from the CLI.
/// * `format` — output format: "reticulum", "json", or "yaml".
/// * `output` — optional file path to write to (stdout if None).
pub fn execute(
    hardware_name: &str,
    interface_name: &str,
    params: &[String],
    format: &str,
    output: Option<&Path>,
) -> ForgeResult<()> {
    // ---- resolve hardware spec ----
    let spec = hardware::spec_by_name(hardware_name).ok_or_else(|| {
        let valid = hardware::all_hardware_names();
        ForgeError::Cli(format!(
            "unknown hardware type '{}'. Valid types: {}",
            hardware_name,
            valid.join(", ")
        ))
    })?;

    // ---- sanitize interface name ----
    let iface_name = sanitize_interface_name(interface_name)?;

    // ---- parse & validate parameters ----
    let parsed = parse_and_validate(spec, params).map_err(|errors| {
        let msg = errors
            .iter()
            .map(|e| format!("  {}: {}", style(&e.param).bold(), e.message))
            .collect::<Vec<_>>()
            .join("\n");
        ForgeError::Validation(format!("parameter validation failed:\n{}", msg))
    })?;

    // ---- build template context ----
    // Collect parameter string values first to ensure they outlive the HashMap borrows.
    let mut param_strings: Vec<(String, String)> = Vec::new();
    for param in &parsed {
        let template_key = map_to_template_key(spec.hw_type, &param.name).to_string();
        let template_val = map_to_template_value(spec.hw_type, &param.name, &param.value);
        param_strings.push((template_key, template_val));
    }
    let iface_type = spec.reticulum_interface_type.to_owned();

    let mut ctx: HashMap<&str, &str> = HashMap::new();
    ctx.insert("interface_name", &iface_name);
    ctx.insert("interface_type", &iface_type);
    for (k, v) in &param_strings {
        ctx.insert(k.as_str(), v.as_str());
    }

    // ---- validate output format ----
    let format = format.to_lowercase();
    match format.as_str() {
        "reticulum" | "json" | "yaml" => {} // valid
        _ => {
            return Err(ForgeError::Cli(format!(
                "unsupported output format '{}'. Use: reticulum, json, or yaml",
                format
            )));
        }
    }

    // ---- progress ----
    let pb = ProgressBar::new(2);
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("static template is valid")
            .tick_strings(&[
                "▹▹▹▹▹",
                "▸▹▹▹▹",
                "▹▸▹▹▹",
                "▹▹▸▹▹",
                "▹▹▹▸▹",
                "▹▹▹▹▸",
                "▪▪▪▪▪",
            ]),
    );

    // ---- render / serialize ----
    let output_content: String = match format.as_str() {
        "reticulum" => {
            pb.set_message("Rendering config template...");
            pb.inc(1);

            let engine = TemplateEngine::new()?;
            let template_name = spec.template_name;
            engine.render_map(template_name, &ctx).inspect_err(|_| {
                pb.finish_with_message("✖ Render failed");
            })?
        }
        "json" => {
            pb.set_message("Serializing to JSON...");
            pb.inc(1);

            let data = OutputJson::from_params(spec.label, interface_name, &parsed);
            serde_json::to_string_pretty(&data).map_err(ForgeError::SerdeJson)?
        }
        "yaml" => {
            pb.set_message("Serializing to YAML...");
            pb.inc(1);

            let data = OutputJson::from_params(spec.label, interface_name, &parsed);
            serde_yaml::to_string(&data).map_err(ForgeError::SerdeYaml)?
        }
        _ => unreachable!(), // validated above
    };

    // ---- write output ----
    pb.inc(1);
    let written_to = if let Some(out_path) = output {
        // Security: validate output path (no directory traversal, must be a file)
        out_path
            .file_name()
            .ok_or_else(|| ForgeError::Validation("output path must be a file name".into()))?;

        let parent = out_path.parent().unwrap_or_else(|| Path::new("."));
        if parent.to_string_lossy().contains("..") {
            return Err(ForgeError::Validation(
                "output path must not contain '..' (directory traversal)".into(),
            ));
        }

        // Use restrictive permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(out_path, &output_content).map_err(ForgeError::Io)?;
            std::fs::set_permissions(out_path, std::fs::Permissions::from_mode(0o644))
                .map_err(ForgeError::Io)?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(out_path, &output_content).map_err(ForgeError::Io)?;
        }

        pb.finish_with_message(format!(
            "{} {} config written to {}",
            style("✔").green(),
            spec.label,
            style(out_path.display()).cyan()
        ));

        out_path.display().to_string()
    } else {
        // Stdout — no file write
        pb.finish_with_message(format!(
            "{} {} config generated",
            style("✔").green(),
            spec.label
        ));
        println!("{}", output_content);
        return Ok(());
    };

    println!("  {} {}", style("📄").bold(), written_to);
    Ok(())
}

/// Sanitize an interface name: only alphanumeric, underscores, and hyphens.
fn sanitize_interface_name(name: &str) -> ForgeResult<String> {
    let sanitized: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();

    if sanitized.is_empty() {
        return Err(ForgeError::Validation(
            "interface name must contain at least one alphanumeric character".into(),
        ));
    }

    if !sanitized.chars().next().is_some_and(|c| c.is_alphabetic()) {
        return Err(ForgeError::Validation(
            "interface name must start with a letter".into(),
        ));
    }

    if sanitized.len() > 64 {
        return Err(ForgeError::Validation(
            "interface name must be 64 characters or fewer".into(),
        ));
    }

    if sanitized != name {
        // Drop a warning if characters were stripped
        eprintln!(
            "{} Interface name '{}' contains invalid characters. Using '{}'",
            style("⚠").yellow(),
            name,
            sanitized
        );
    }

    Ok(sanitized)
}

/// Map a canonical parameter name to its template variable name.
fn map_to_template_key(hw: hardware::HardwareType, param: &str) -> &str {
    use hardware::HardwareType::*;
    match (hw, param) {
        (RNodeLora, "freq") => "freq_hz",
        (RNodeLora, "bw") => "bw_hz",
        // most params use their canonical name directly
        _ => param,
    }
}

/// Convert a parameter value from display format to template-ready format.
fn map_to_template_value(hw: hardware::HardwareType, param: &str, value: &str) -> String {
    use hardware::HardwareType::*;
    match (hw, param) {
        // Convert "868mhz" → "868000000" for Reticulum config
        (RNodeLora, "freq") => hardware::parse_frequency(value)
            .map(|h| h.to_string())
            .unwrap_or_else(|_| value.to_string()),
        // Convert "125khz" → "125000"
        (RNodeLora, "bw") => hardware::parse_bandwidth(value)
            .map(|h| h.to_string())
            .unwrap_or_else(|_| value.to_string()),
        // Pass through everything else
        _ => value.to_string(),
    }
}

/// JSON/YAML output structure.
#[derive(Serialize)]
struct OutputJson {
    interface: InterfaceOutput,
    metadata: MetadataOutput,
}

#[derive(Serialize)]
struct InterfaceOutput {
    name: String,
    hardware_type: String,
    params: HashMap<String, String>,
}

#[derive(Serialize)]
struct MetadataOutput {
    generator: &'static str,
    version: &'static str,
}

impl OutputJson {
    fn from_params(hardware_label: &str, name: &str, params: &[Param]) -> Self {
        let mut param_map = HashMap::new();
        for p in params {
            param_map.insert(p.name.clone(), p.value.clone());
        }
        OutputJson {
            interface: InterfaceOutput {
                name: name.to_string(),
                hardware_type: hardware_label.to_string(),
                params: param_map,
            },
            metadata: MetadataOutput {
                generator: "Reticulum Forge",
                version: env!("CARGO_PKG_VERSION"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_interface_name_valid() {
        assert_eq!(sanitize_interface_name("lora_0").unwrap(), "lora_0");
        assert_eq!(
            sanitize_interface_name("backbone-link").unwrap(),
            "backbone-link"
        );
    }

    #[test]
    fn test_sanitize_interface_name_strips_bad_chars() {
        let result = sanitize_interface_name("my iface; rm -rf /").unwrap();
        assert_eq!(result, "myifacerm-rf"); // spaces, semicolons, slashes stripped; hyphens kept
    }

    #[test]
    fn test_sanitize_interface_name_empty() {
        assert!(sanitize_interface_name("---").is_err());
    }

    #[test]
    fn test_sanitize_interface_name_too_long() {
        let long = "a".repeat(65);
        assert!(sanitize_interface_name(&long).is_err());
    }

    #[test]
    fn test_sanitize_interface_name_must_start_with_letter() {
        assert!(sanitize_interface_name("0interface").is_err());
    }

    #[test]
    fn test_output_json_structure() {
        let params = vec![
            Param {
                name: "freq".into(),
                value: "868000000".into(),
            },
            Param {
                name: "sf".into(),
                value: "10".into(),
            },
        ];
        let output = OutputJson::from_params("RNode LoRa", "lora_0", &params);
        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("868000000"));
        assert!(json.contains("RNode LoRa"));
        assert!(json.contains("Reticulum Forge"));
    }
}
