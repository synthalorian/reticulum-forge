//! Hardware database — validated parameter ranges per device type.
//!
//! Each hardware type defines its supported parameters, types, and validation
//! rules. This is the source of truth for `forge generate`.
//!
//! # Supported Hardware
//! - RNode LoRa (long-range radio)
//! - Serial TNC / KISS (serial port modem)
//! - TCP Client (outbound internet/LAN peering)
//! - TCP Server (inbound listener)
//! - AutoInterface (local multicast discovery)

use std::fmt;

/// Supported hardware/interface types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareType {
    RNodeLora,
    Serial,
    TcpClient,
    TcpServer,
    AutoInterface,
}

impl fmt::Display for HardwareType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HardwareType::RNodeLora => write!(f, "rnode-lora"),
            HardwareType::Serial => write!(f, "serial"),
            HardwareType::TcpClient => write!(f, "tcp-client"),
            HardwareType::TcpServer => write!(f, "tcp-server"),
            HardwareType::AutoInterface => write!(f, "auto"),
        }
    }
}

/// Validation rule for a parameter value.
#[derive(Debug, Clone)]
pub enum ParamValidation {
    /// Free-form string (still sanitized — alphanumeric, underscores, hyphens).
    String,
    /// One of a fixed set of choices (case-insensitive match).
    Choice(&'static [&'static str]),
    /// Integer in [min, max].
    Int { min: i64, max: i64 },
    /// Float in [min, max].
    #[allow(dead_code)]
    Float { min: f64, max: f64 },
    /// Frequency string like "868mhz", "433mhz", "915mhz" — parsed to Hz integer.
    Frequency,
    /// Bandwidth string like "125khz", "250khz" — parsed to Hz integer.
    #[allow(dead_code)]
    Bandwidth,
    /// Port number 1–65535.
    Port,
    /// File-system path — checked for traversal attacks.
    Path,
}

/// Definition of one configurable parameter.
#[derive(Debug, Clone)]
pub struct ParamDef {
    /// CLI flag name (e.g. "freq", "bw").
    pub name: &'static str,
    /// Human-readable description.
    #[allow(dead_code)]
    pub description: &'static str,
    /// Default value if not supplied.
    pub default: Option<&'static str>,
    /// Whether the parameter is required.
    pub required: bool,
    /// Validation rule.
    pub validation: ParamValidation,
}

/// Full specification of a hardware interface type.
#[derive(Debug, Clone)]
pub struct HardwareSpec {
    pub hw_type: HardwareType,
    /// Display label (e.g. "RNode LoRa").
    pub label: &'static str,
    #[allow(dead_code)]
    pub description: &'static str,
    pub template_name: &'static str,
    /// Parameters accepted by this hardware type.
    pub parameters: &'static [ParamDef],
    /// Reticulum interface type string (e.g. "RNodeInterface").
    pub reticulum_interface_type: &'static str,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

mod auto_interface;
mod rnode;
mod serial;
mod tcp;

/// All registered hardware specs.
pub(crate) static ALL_SPECS: &[&HardwareSpec] = &[
    &rnode::RNODE_SPEC,
    &serial::SERIAL_SPEC,
    &tcp::TCP_CLIENT_SPEC,
    &tcp::TCP_SERVER_SPEC,
    &auto_interface::AUTO_INTERFACE_SPEC,
];

/// Look up a hardware spec by its string name (case-insensitive).
pub fn spec_by_name(name: &str) -> Option<&'static HardwareSpec> {
    let lower = name.to_lowercase();
    ALL_SPECS.iter().copied().find(|s| {
        let s_name = s.hw_type.to_string().to_lowercase();
        s_name == lower || s.label.to_lowercase().contains(&lower)
    })
}

/// Look up a hardware spec by its enum variant.
#[allow(dead_code)]
pub fn spec_by_type(hw: HardwareType) -> Option<&'static HardwareSpec> {
    ALL_SPECS.iter().copied().find(|s| s.hw_type == hw)
}

/// List all registered hardware types as display strings.
pub fn all_hardware_names() -> Vec<&'static str> {
    ALL_SPECS.iter().map(|s| s.label).collect()
}

// ---------------------------------------------------------------------------
// Parameter parsing & validation
// ---------------------------------------------------------------------------

/// A parsed and validated key-value parameter.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub value: String,
}

/// Error returned when parameter validation fails.
#[derive(Debug, Clone)]
pub struct ParamError {
    pub param: String,
    pub message: String,
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parameter '{}': {}", self.param, self.message)
    }
}

/// Parse --param "key=value" strings and validate against the given spec.
///
/// Returns `Ok(parsed_params)` on success, or `Err(errors)` with ALL
/// validation failures collected (not just the first one).
pub fn parse_and_validate(
    spec: &HardwareSpec,
    raw: &[String],
) -> Result<Vec<Param>, Vec<ParamError>> {
    let mut errors: Vec<ParamError> = Vec::new();
    let mut parsed: Vec<Param> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Build a lookup for fast param-def access.
    let def_map: std::collections::HashMap<&str, &ParamDef> =
        spec.parameters.iter().map(|d| (d.name, d)).collect();

    // Parse raw key=value pairs.
    for pair in raw {
        let (key, val) = match pair.split_once('=') {
            Some((k, v)) => (k.trim().to_lowercase(), v.trim()),
            None => {
                errors.push(ParamError {
                    param: pair.clone(),
                    message: "expected key=value format (e.g. freq=868mhz)".into(),
                });
                continue;
            }
        };

        let def = match def_map.get(key.as_str()) {
            Some(d) => d,
            None => {
                errors.push(ParamError {
                    param: key.clone(),
                    message: format!(
                        "unknown parameter '{}' for {}. Allowed: {}",
                        key,
                        spec.label,
                        spec.parameters
                            .iter()
                            .map(|p| p.name)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
                continue;
            }
        };

        // Validate
        if let Err(msg) = validate_value(def, val) {
            errors.push(ParamError {
                param: key.clone(),
                message: msg,
            });
            continue;
        }

        if !seen.insert(key.clone()) {
            errors.push(ParamError {
                param: key.clone(),
                message: "duplicate parameter".into(),
            });
            continue;
        }

        parsed.push(Param {
            name: key,
            value: val.to_string(),
        });
    }

    // Check for missing required params.
    for def in spec.parameters {
        if def.required && !seen.contains(def.name) {
            let default_hint = match def.default {
                Some(d) => format!(" (default: {})", d),
                None => String::new(),
            };
            errors.push(ParamError {
                param: def.name.to_string(),
                message: format!("required parameter not provided{}", default_hint),
            });
        }
    }

    if errors.is_empty() {
        // Fill in defaults for any missing optional params.
        for def in spec.parameters {
            if !seen.contains(def.name) {
                if let Some(default) = def.default {
                    parsed.push(Param {
                        name: def.name.to_string(),
                        value: default.to_string(),
                    });
                }
            }
        }
        Ok(parsed)
    } else {
        Err(errors)
    }
}

fn validate_value(def: &ParamDef, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("value must not be empty".into());
    }

    match def.validation {
        ParamValidation::String => {
            // Block path-traversal chars and shell metacharacters.
            if value.contains('/')
                || value.contains('\\')
                || value.contains(';')
                || value.contains('`')
                || value.contains('$')
                || value.contains('|')
                || value.contains('>')
                || value.contains('<')
            {
                return Err("contains forbidden characters (/, \\, ;, `, $, |, <, >)".into());
            }
        }
        ParamValidation::Choice(choices) => {
            let lower_val = value.to_lowercase();
            if !choices.iter().any(|c| c.to_lowercase() == lower_val) {
                return Err(format!(
                    "invalid choice '{}'. Allowed: {}",
                    value,
                    choices.join(", ")
                ));
            }
        }
        ParamValidation::Int { min, max } => {
            let n: i64 = value
                .parse()
                .map_err(|_| format!("'{}' is not a valid integer", value))?;
            if n < min || n > max {
                return Err(format!("must be between {} and {}", min, max));
            }
        }
        ParamValidation::Float { min, max } => {
            let n: f64 = value
                .parse()
                .map_err(|_| format!("'{}' is not a valid number", value))?;
            if n < min || n > max {
                return Err(format!("must be between {} and {}", min, max));
            }
        }
        ParamValidation::Frequency => {
            let hz = parse_frequency(value).map_err(|_| {
                "invalid frequency format. Use e.g. 868mhz, 433mhz, 915mhz, or 867000000 (Hz)"
                    .to_string()
            })?;
            // Valid Reticulum bands: typically 433MHz, 868MHz, 915MHz, and some others
            if !(100_000_000..=10_000_000_000u64).contains(&hz) {
                return Err("frequency must be between 100 MHz and 10 GHz".into());
            }
        }
        ParamValidation::Bandwidth => {
            let hz = parse_bandwidth(value).map_err(|_| {
                "invalid bandwidth format. Use e.g. 125khz, 250khz, 500khz".to_string()
            })?;
            if !(7_800..=2_000_000u64).contains(&hz) {
                return Err("bandwidth must be between 7.8 kHz and 2 MHz".into());
            }
        }
        ParamValidation::Port => {
            let n: u16 = value
                .parse()
                .map_err(|_| format!("'{}' is not a valid port number", value))?;
            if n == 0 {
                return Err("port 0 is reserved".into());
            }
        }
        ParamValidation::Path => {
            if value.contains("..") {
                return Err("path must not contain '..' (directory traversal)".into());
            }
            if value.contains('/') || value.contains('\\') {
                return Err("use just the device name (e.g. ttyUSB0), not a full path".into());
            }
            if value.starts_with('/') || value.starts_with('~') {
                return Err(
                    "absolute paths are not allowed — use a device name like ttyUSB0".into(),
                );
            }
        }
    }
    Ok(())
}

/// Parse frequency strings like "868mhz", "433mhz", "915mhz" to Hz.
pub fn parse_frequency(s: &str) -> Result<u64, ()> {
    let s = s.trim().to_lowercase();
    if let Some(rest) = s.strip_suffix("mhz") {
        let mhz: f64 = rest.trim().parse().map_err(|_| ())?;
        Ok((mhz * 1_000_000.0) as u64)
    } else if let Some(rest) = s.strip_suffix("khz") {
        let khz: f64 = rest.trim().parse().map_err(|_| ())?;
        Ok((khz * 1_000.0) as u64)
    } else if let Some(rest) = s.strip_suffix("hz") {
        let hz: f64 = rest.trim().parse().map_err(|_| ())?;
        Ok(hz as u64)
    } else {
        // Plain number — treat as Hz
        s.parse::<u64>().map_err(|_| ())
    }
}

/// Parse bandwidth strings like "125khz", "250khz", "500khz" to Hz.
pub fn parse_bandwidth(s: &str) -> Result<u64, ()> {
    let s = s.trim().to_lowercase();
    if let Some(rest) = s.strip_suffix("khz") {
        let khz: f64 = rest.trim().parse().map_err(|_| ())?;
        Ok((khz * 1_000.0) as u64)
    } else if let Some(rest) = s.strip_suffix("mhz") {
        let mhz: f64 = rest.trim().parse().map_err(|_| ())?;
        Ok((mhz * 1_000_000.0) as u64)
    } else if let Some(rest) = s.strip_suffix("hz") {
        let hz: f64 = rest.trim().parse().map_err(|_| ())?;
        Ok(hz as u64)
    } else {
        s.parse::<u64>().map_err(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frequency() {
        assert_eq!(parse_frequency("868mhz").unwrap(), 868_000_000);
        assert_eq!(parse_frequency("433mhz").unwrap(), 433_000_000);
        assert_eq!(parse_frequency("915mhz").unwrap(), 915_000_000);
        assert_eq!(parse_frequency("867000000").unwrap(), 867_000_000);
    }

    #[test]
    fn test_parse_bandwidth() {
        assert_eq!(parse_bandwidth("125khz").unwrap(), 125_000);
        assert_eq!(parse_bandwidth("250khz").unwrap(), 250_000);
        assert_eq!(parse_bandwidth("500khz").unwrap(), 500_000);
    }

    #[test]
    fn test_parse_frequency_invalid() {
        assert!(parse_frequency("not-a-freq").is_err());
        assert!(parse_frequency("100ghz").is_err()); // ghz not supported directly
    }

    #[test]
    fn test_spec_by_name() {
        let spec = spec_by_name("rnode-lora").unwrap();
        assert_eq!(spec.hw_type, HardwareType::RNodeLora);

        let spec = spec_by_name("serial").unwrap();
        assert_eq!(spec.hw_type, HardwareType::Serial);

        let spec = spec_by_name("tcp-client").unwrap();
        assert_eq!(spec.hw_type, HardwareType::TcpClient);
    }

    #[test]
    fn test_spec_by_name_case_insensitive() {
        let spec = spec_by_name("RNODE-LORA").unwrap();
        assert_eq!(spec.hw_type, HardwareType::RNodeLora);
    }

    #[test]
    fn test_validate_rnode_params() {
        let spec = spec_by_name("rnode-lora").unwrap();
        let raw = vec![
            "freq=868mhz".into(),
            "bw=125khz".into(),
            "sf=10".into(),
            "tx_power=17".into(),
        ];
        let result = parse_and_validate(spec, &raw);
        assert!(result.is_ok(), "validation failed: {:?}", result.err());
        let params = result.unwrap();
        assert!(params.iter().any(|p| p.name == "freq"));
        assert!(params.iter().any(|p| p.name == "tx_power"));
    }

    #[test]
    fn test_validate_unknown_param() {
        let spec = spec_by_name("serial").unwrap();
        let raw = vec!["freq=868mhz".into()]; // serial doesn't have freq
        let result = parse_and_validate(spec, &raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_int_out_of_range() {
        let spec = spec_by_name("rnode-lora").unwrap();
        let raw = vec!["sf=99".into(), "freq=868mhz".into()];
        let result = parse_and_validate(spec, &raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_traversal() {
        let spec = spec_by_name("serial").unwrap();
        let raw = vec!["port=../../etc/passwd".into()];
        let result = parse_and_validate(spec, &raw);
        assert!(result.is_err());
    }
}
