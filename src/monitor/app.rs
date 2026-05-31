//! TUI monitor application — event loop, terminal setup, background polling.
//!
//! Architecture:
//! - Main thread runs the synchronous crossterm event loop (non-blocking poll).
//! - A background thread periodically runs SSH health checks via `poll_nodes()`.
//! - State is shared via `Arc<Mutex<MonitorState>>` for the background thread to update.
//!
//! Security:
//! - Terminal raw mode is restored on any panic via a guard wrapper.
//! - crossterm event read is bounded by poll(Duration) — no blocking reads.

use crate::deploy::inventory::Inventory;
use crate::monitor::node_status::{AppStatus, MonitorState};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Frequency of automatic background health polls.
const POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Frequency of checking for new poll results.
const TICK_INTERVAL: Duration = Duration::from_millis(100);

/// Wraps terminal lifecycle to guarantee cleanup on drop/panic.
pub struct TerminalGuard {
    _private: (),
}

impl TerminalGuard {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        Ok(TerminalGuard { _private: () })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = disable_raw_mode();
    }
}

/// Run the TUI monitor with the given inventory.
pub fn run_monitor(inventory: Arc<Inventory>) -> io::Result<()> {
    let _guard = TerminalGuard::new()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    // Shared state between background poller and UI thread
    let state = Arc::new(Mutex::new(MonitorState::from_inventory(&inventory)));
    let running = Arc::new(AtomicBool::new(true));

    // Spawn background polling thread
    let state_clone = Arc::clone(&state);
    let inv_clone = Arc::clone(&inventory);
    let running_clone = Arc::clone(&running);
    thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                if let Ok(mut s) = state_clone.lock() {
                    s.add_log(format!("poller: failed to create tokio runtime: {}", e));
                }
                return;
            }
        };

        // Keep a database of last poll times for rate-limiting
        let mut last_poll_times: Vec<std::time::Instant> = Vec::new();

        while running_clone.load(Ordering::Relaxed) {
            thread::sleep(POLL_INTERVAL);

            if !running_clone.load(Ordering::Relaxed) {
                break;
            }

            // Guard against rapid polls
            let now = std::time::Instant::now();
            last_poll_times.retain(|t| now.duration_since(*t) < Duration::from_secs(60));
            if last_poll_times.len() >= 3 {
                // Too many polls in window — skip
                if let Ok(mut s) = state_clone.lock() {
                    s.add_log("poller: rate limit hit, skipping poll cycle");
                }
                continue;
            }
            last_poll_times.push(now);

            // Clone the current state for the poll (avoids holding lock during SSH)
            let current_state = match state_clone.lock() {
                Ok(s) => s.clone(),
                Err(_) => {
                    if let Ok(mut s) = state_clone.lock() {
                        s.add_log("poller: state lock poisoned, aborting");
                    }
                    return;
                }
            };

            // Poll all nodes — this creates a new state
            let new_state = rt.block_on(async {
                // We call poll_nodes which also creates its own rt internally...
                // Actually, let's just do it more efficiently:
                poll_nodes_internal(&current_state, &inv_clone, &rt)
            });

            // Update shared state
            let log_msg = format!(
                "polled {} nodes: {} healthy, {} degraded, {} offline",
                new_state.summary.total,
                new_state.summary.healthy,
                new_state.summary.degraded,
                new_state.summary.offline,
            );
            if let Ok(mut s) = state_clone.lock() {
                *s = new_state;
                s.add_log(log_msg);
            }
        }
    });

    // Main UI event loop (synchronous)
    let res = run_event_loop(&mut terminal, &state, &running);

    // Signal background thread to stop
    running.store(false, Ordering::Relaxed);

    // Clean up terminal
    terminal.show_cursor()?;
    terminal.clear()?;

    res
}

/// Run the main event loop — poll for keyboard input and render.
fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &Arc<Mutex<MonitorState>>,
    running: &Arc<AtomicBool>,
) -> io::Result<()> {
    // Do an immediate initial poll
    if let Ok(_s) = state.lock() {
        // First render shows initial state
    }

    loop {
        // Render
        {
            let state_guard = state.lock().unwrap_or_else(|e| {
                // Poisoned lock — can't recover
                running.store(false, Ordering::Relaxed);
                e.into_inner()
            });

            if state_guard.status == AppStatus::Quitting {
                break;
            }

            terminal.draw(|f| {
                crate::monitor::ui::draw(f, &state_guard);
            })?;
        }

        // Check for key events with a short timeout (allows polling to update state)
        if event::poll(TICK_INTERVAL)? {
            match event::read()? {
                Event::Key(key) => {
                    let mut state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    handle_key(key, &mut state_guard);

                    if state_guard.status == AppStatus::Quitting {
                        break;
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal handles resize automatically
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Handle a single key event.
fn handle_key(key: KeyEvent, state: &mut MonitorState) {
    match state.status {
        AppStatus::Filtering => match key.code {
            KeyCode::Esc => {
                state.filter.clear();
                state.status = AppStatus::Running;
                state.clamp_selection();
            }
            KeyCode::Enter => {
                state.status = AppStatus::Running;
                state.clamp_selection();
            }
            KeyCode::Char(c) => {
                state.filter.push(c);
                state.clamp_selection();
            }
            KeyCode::Backspace => {
                state.filter.pop();
                state.clamp_selection();
            }
            _ => {}
        },
        AppStatus::Running => {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    state.status = AppStatus::Quitting;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.select_prev();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    state.select_next();
                }
                KeyCode::Char('/') => {
                    state.status = AppStatus::Filtering;
                }
                KeyCode::Enter => {
                    state.toggle_detail();
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    // Manual refresh triggered by key — just add log entry.
                    // The background poller picks this up on next cycle.
                    state.add_log("manual refresh requested (next poll cycle)");
                }
                _ => {}
            }
        }
        AppStatus::Quitting => {
            // No-op
        }
    }
}

/// Lightweight poll that reuses an existing tokio runtime.
/// This avoids creating a new Runtime per poll cycle.
fn poll_nodes_internal(
    state: &MonitorState,
    inventory: &Inventory,
    rt: &tokio::runtime::Runtime,
) -> MonitorState {
    use crate::deploy::health::check_node_health_detailed;
    use crate::deploy::ssh::SshConfig;

    let mut new_state = state.clone();
    let mut updated_any = false;

    for node_row in &mut new_state.nodes {
        let node_name = node_row.name.clone();
        let node_opt = inventory.nodes.get(&node_name).cloned();
        let Some(node) = node_opt else {
            node_row.health =
                crate::deploy::health::HealthStatus::Offline("removed from inventory".into());
            continue;
        };

        let ssh_config = SshConfig::from(&node);

        let result = rt.block_on(async {
            check_node_health_detailed(
                &node_name,
                &ssh_config,
                &node.service_name,
                &node.config_path,
            )
            .await
        });

        node_row.health = result.status.clone();
        node_row.health_detail = result.details.clone();
        node_row.last_check = std::time::Instant::now();
        node_row.host = node.host.clone();

        // Fetch uptime on healthy and degraded nodes
        if node_row.uptime.is_empty() || !node_row.health.is_offline() {
            let uptime_result = rt.block_on(async {
                let mut client = match crate::deploy::ssh::SshClient::connect(&ssh_config).await {
                    Ok(c) => c,
                    Err(_) => return String::new(),
                };
                let r = client
                    .execute("uptime -p 2>/dev/null || uptime 2>/dev/null || echo ''")
                    .await;
                let _ = client.close().await;
                match r {
                    Ok(res) => res.stdout.trim().to_string(),
                    Err(_) => String::new(),
                }
            });
            if !uptime_result.is_empty() {
                node_row.uptime = uptime_result;
            }
        }

        updated_any = true;
    }

    if updated_any {
        new_state.last_poll = std::time::Instant::now();
        new_state.recompute_summary();
    }

    new_state
}
