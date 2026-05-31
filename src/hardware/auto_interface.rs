//! Hardware spec: AutoInterface — local peer discovery via multicast.

use crate::hardware::{HardwareSpec, HardwareType, ParamDef, ParamValidation};

/// AutoInterface — automatic peer discovery on the local network.
///
/// Uses UDP multicast to discover and connect to nearby Reticulum peers
/// without any configuration. Ideal for local mesh networks on the same
/// LAN segment. The group_id isolates different Reticulum networks on
/// the same broadcast domain.
pub static AUTO_INTERFACE_SPEC: HardwareSpec = HardwareSpec {
    hw_type: HardwareType::AutoInterface,
    label: "AutoInterface",
    description: "Automatic peer discovery on the local network via UDP multicast",
    template_name: "auto_interface_config",
    reticulum_interface_type: "AutoInterface",
    parameters: &[
        ParamDef {
            name: "group_id",
            description: "Isolation group ID (shared by peers on the same network segment)",
            default: Some("forge_default"),
            required: false,
            validation: ParamValidation::String,
        },
        ParamDef {
            name: "group_port",
            description: "Multicast port (1024-65535)",
            default: Some("42420"),
            required: false,
            validation: ParamValidation::Port,
        },
        ParamDef {
            name: "discovery_interval",
            description: "Discovery announce interval in seconds (10-3600)",
            default: Some("120"),
            required: false,
            validation: ParamValidation::Int { min: 10, max: 3600 },
        },
        ParamDef {
            name: "multicast_address",
            description: "Multicast address (e.g. 224.0.0.1, ff02::1)",
            default: Some("224.0.0.1"),
            required: false,
            validation: ParamValidation::String,
        },
    ],
};
