//! TUI monitor module — real-time node status dashboard.
//!
//! Provides a terminal UI for viewing and monitoring the health of
//! Reticulum mesh nodes managed by the forge inventory.
//!
//! # Design
//! - Synchronous crossterm event loop on the main thread.
//! - Background thread for periodic SSH health polling.
//! - Shared state via `Arc<Mutex<MonitorState>>`.

pub mod app;
pub mod node_status;
pub mod ui;
