//! Hardware spec: RNode LoRa radio interface.

use crate::hardware::{HardwareSpec, HardwareType, ParamDef, ParamValidation};

/// RNode LoRa — long-range radio interface for Reticulum.
///
/// Valid frequency bands: 433 MHz, 868 MHz (EU), 915 MHz (US/AU).
/// Spreading factor 7–12, bandwidth 62.5–500 kHz, TX power 0–17 dBm.
pub static RNODE_SPEC: HardwareSpec = HardwareSpec {
    hw_type: HardwareType::RNodeLora,
    label: "RNode LoRa",
    description: "RNode LoRa radio interface for long-range mesh links",
    template_name: "rnode_config",
    reticulum_interface_type: "RNodeInterface",
    parameters: &[
        ParamDef {
            name: "freq",
            description: "Frequency band (e.g. 868mhz, 433mhz, 915mhz)",
            default: None,
            required: true,
            validation: ParamValidation::Frequency,
        },
        ParamDef {
            name: "bw",
            description: "Bandwidth (62.5khz, 125khz, 250khz, 500khz)",
            default: Some("125khz"),
            required: false,
            validation: ParamValidation::Choice(&["62.5khz", "125khz", "250khz", "500khz"]),
        },
        ParamDef {
            name: "sf",
            description: "Spreading factor (7-12)",
            default: Some("10"),
            required: false,
            validation: ParamValidation::Int { min: 7, max: 12 },
        },
        ParamDef {
            name: "tx_power",
            description: "Transmit power in dBm (0-17)",
            default: Some("17"),
            required: false,
            validation: ParamValidation::Int { min: 0, max: 17 },
        },
        ParamDef {
            name: "coding_rate",
            description: "Coding rate (5-8)",
            default: Some("6"),
            required: false,
            validation: ParamValidation::Int { min: 5, max: 8 },
        },
        ParamDef {
            name: "port",
            description: "Serial port device (e.g. ttyUSB0)",
            default: Some("ttyUSB0"),
            required: false,
            validation: ParamValidation::Path,
        },
    ],
};
