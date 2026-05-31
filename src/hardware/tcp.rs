//! Hardware specs: TCP client and server interfaces.

use crate::hardware::{HardwareSpec, HardwareType, ParamDef, ParamValidation};

/// TCP Client — connects to a remote Reticulum peer outbound.
///
/// This is the usual way to peer over the internet or a LAN.
/// The target_host is validated for basic safety (no protocol prefixes,
/// no shell metacharacters).
pub static TCP_CLIENT_SPEC: HardwareSpec = HardwareSpec {
    hw_type: HardwareType::TcpClient,
    label: "TCP Client",
    description: "TCP client interface — outbound connection to a remote peer",
    template_name: "tcp_client_config",
    reticulum_interface_type: "TCPClientInterface",
    parameters: &[
        ParamDef {
            name: "target_host",
            description: "Remote host address (IP or hostname)",
            default: None,
            required: true,
            validation: ParamValidation::String,
        },
        ParamDef {
            name: "target_port",
            description: "Remote port (1-65535)",
            default: Some("37428"),
            required: false,
            validation: ParamValidation::Port,
        },
        ParamDef {
            name: "keepalive",
            description: "Keepalive interval in seconds (10-3600)",
            default: Some("300"),
            required: false,
            validation: ParamValidation::Int { min: 10, max: 3600 },
        },
        ParamDef {
            name: "timeout",
            description: "Connection timeout in seconds (1-120)",
            default: Some("30"),
            required: false,
            validation: ParamValidation::Int { min: 1, max: 120 },
        },
    ],
};

/// TCP Server — listens for inbound Reticulum peer connections.
///
/// Binds to a local port and accepts connections from remote peers.
/// For security, does NOT support binding to '0.0.0.0' by default —
/// user must explicitly choose their listen address.
pub static TCP_SERVER_SPEC: HardwareSpec = HardwareSpec {
    hw_type: HardwareType::TcpServer,
    label: "TCP Server",
    description: "TCP server interface — listen for inbound peer connections",
    template_name: "tcp_server_config",
    reticulum_interface_type: "TCPServerInterface",
    parameters: &[
        ParamDef {
            name: "listen_address",
            description: "Local address to bind (e.g. 0.0.0.0, 127.0.0.1, ::1)",
            default: Some("127.0.0.1"),
            required: false,
            validation: ParamValidation::String,
        },
        ParamDef {
            name: "listen_port",
            description: "Local port to bind (1-65535)",
            default: Some("37428"),
            required: false,
            validation: ParamValidation::Port,
        },
        ParamDef {
            name: "allowed_hosts",
            description:
                "Comma-separated CIDR whitelist (e.g. 10.0.0.0/8,192.168.0.0/16). Empty = any",
            default: Some(""),
            required: false,
            validation: ParamValidation::String,
        },
        ParamDef {
            name: "require_encryption",
            description: "Require encrypted links (yes/no)",
            default: Some("yes"),
            required: false,
            validation: ParamValidation::Choice(&["yes", "no"]),
        },
    ],
};
