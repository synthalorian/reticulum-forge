use crate::deploy::inventory::Inventory;
use crate::error::{ForgeError, ForgeResult};
use crate::monitor::app::run_monitor;
use std::sync::Arc;

/// Run the TUI monitor dashboard.
///
/// Displays all nodes from the inventory with live health status,
/// event log, and keyboard navigation.
///
/// # Security
/// - Inventory path is validated for directory traversal before loading.
/// - Input length is bounded by the CLI layer.
pub fn execute(inventory_path: &str) -> ForgeResult<()> {
    // Validate path (no directory traversal)
    if inventory_path.contains("..") {
        return Err(ForgeError::Validation(
            "inventory path must not contain '..' (directory traversal)".into(),
        ));
    }

    let inventory = Inventory::load(inventory_path)?;
    run_monitor(Arc::new(inventory))?;
    Ok(())
}
