//! Hardware spec: Serial TNC / KISS interface.

use crate::hardware::{HardwareSpec, HardwareType, ParamDef, ParamValidation};

/// Serial TNC — connects to a serial port running KISS protocol.
///
/// Supports standard baud rates (1200–115200) and flow control options.
/// The port parameter is a device name (not a full path) to prevent
/// directory traversal attacks.
pub static SERIAL_SPEC: HardwareSpec = HardwareSpec {
    hw_type: HardwareType::Serial,
    label: "Serial TNC",
    description: "Serial port interface for TNC/KISS modems",
    template_name: "serial_config",
    reticulum_interface_type: "SerialInterface",
    parameters: &[
        ParamDef {
            name: "port",
            description: "Serial port device (e.g. ttyUSB0, ttyAMA0)",
            default: Some("ttyUSB0"),
            required: false,
            validation: ParamValidation::Path,
        },
        ParamDef {
            name: "baud",
            description: "Baud rate (1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200)",
            default: Some("115200"),
            required: false,
            validation: ParamValidation::Choice(&[
                "1200", "2400", "4800", "9600", "19200", "38400", "57600", "115200",
            ]),
        },
        ParamDef {
            name: "flow_control",
            description: "Flow control method (none, rtscts, xonxoff)",
            default: Some("none"),
            required: false,
            validation: ParamValidation::Choice(&["none", "rtscts", "xonxoff"]),
        },
        ParamDef {
            name: "kiss_tx_delay",
            description: "KISS TX delay in milliseconds (10-500)",
            default: Some("50"),
            required: false,
            validation: ParamValidation::Int { min: 10, max: 500 },
        },
        ParamDef {
            name: "kiss_persistence",
            description: "KISS persistence (0-255)",
            default: Some("63"),
            required: false,
            validation: ParamValidation::Int { min: 0, max: 255 },
        },
        ParamDef {
            name: "kiss_slot_time",
            description: "KISS slot time in milliseconds (10-500)",
            default: Some("100"),
            required: false,
            validation: ParamValidation::Int { min: 10, max: 500 },
        },
    ],
};
