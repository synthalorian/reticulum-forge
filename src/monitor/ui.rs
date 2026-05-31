use crate::monitor::node_status::{AppStatus, MonitorState};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Render the full monitor UI into the given frame.
pub fn draw(frame: &mut Frame<'_>, state: &MonitorState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(6),
        ])
        .split(area);

    render_summary_bar(frame, state, chunks[0]);
    render_body(frame, state, chunks[1]);
    render_event_log(frame, state, chunks[2]);
}

fn render_summary_bar(frame: &mut Frame<'_>, state: &MonitorState, area: Rect) {
    let title = format!("  {} NODES  ", state.summary.total);

    let mut spans = vec![
        Span::styled(
            format!(" ● {} healthy ", state.summary.healthy),
            Style::default().fg(Color::Green),
        ),
        Span::styled(
            format!(" ● {} degraded ", state.summary.degraded),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(
            format!(" ● {} offline ", state.summary.offline),
            Style::default().fg(Color::Red),
        ),
    ];

    if state.status == AppStatus::Filtering {
        let filter_text = format!("  [FILTER: {}_]", state.filter);
        spans.push(Span::styled(
            filter_text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    } else if !state.filter.is_empty() {
        let filter_text = format!("  [filter: {}]", state.filter);
        spans.push(Span::styled(
            filter_text,
            Style::default().fg(Color::DarkGray),
        ));
    }

    let text = Text::from(Line::from(spans));
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    &title,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

fn render_body(frame: &mut Frame<'_>, state: &MonitorState, area: Rect) {
    if state.nodes.is_empty() {
        let empty = Paragraph::new("No nodes in inventory.")
            .block(Block::default().borders(Borders::ALL).title(" Nodes "))
            .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    let right_width = if state.filtered_nodes().iter().any(|n| n.expanded) {
        25
    } else {
        10
    };

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(30), Constraint::Length(right_width)])
        .split(area);

    render_node_list(frame, state, body_chunks[0]);
    render_detail_panel(frame, state, body_chunks[1]);
}

fn render_node_list(frame: &mut Frame<'_>, state: &MonitorState, area: Rect) {
    let filtered = state.filtered_nodes();
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let status_char = if row.health.is_healthy() {
                "●"
            } else if row.health.is_degraded() {
                "◐"
            } else {
                "○"
            };
            let status_color = if row.health.is_healthy() {
                Color::Green
            } else if row.health.is_degraded() {
                Color::Yellow
            } else {
                Color::Red
            };

            let prefix = if i == state.selected_index {
                "▶"
            } else {
                " "
            };
            let label = format!("{} {} {}  {}", prefix, status_char, row.name, row.host);

            let style = if i == state.selected_index {
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(status_color)
            };

            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Nodes ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(list, area);
}

fn render_detail_panel(frame: &mut Frame<'_>, state: &MonitorState, area: Rect) {
    let filtered = state.filtered_nodes();
    let selected = filtered.get(state.selected_index);

    let content: Vec<Line> = match selected {
        None => vec![Line::from(Span::raw("No node selected."))],
        Some(row) if !row.expanded => {
            let status_str = if row.health.is_healthy() {
                "Healthy"
            } else if row.health.is_degraded() {
                "Degraded"
            } else {
                "Offline"
            };
            vec![
                Line::from(Span::styled(
                    format!(" {}", row.name),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::raw("")),
                Line::from(Span::raw(format!(" Host:  {}", row.host))),
                Line::from(Span::raw(format!(" State: {}", status_str))),
                Line::from(Span::raw(format!(" Uptime: {}", row.uptime))),
                Line::from(Span::raw("")),
                Line::from(Span::styled(
                    " <Enter> for details ",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        Some(row) => {
            vec![
                Line::from(Span::styled(
                    format!(" {}", row.name),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::raw("")),
                Line::from(Span::raw(format!(" Host:    {}", row.host))),
                Line::from(Span::raw(format!(
                    " Status:  {}",
                    health_label(&row.health)
                ))),
                Line::from(Span::raw(format!(" Uptime:  {}", row.uptime))),
                Line::from(Span::raw(format!(" Detail:  {}", row.health_detail))),
            ]
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Detail ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(Text::from(content))
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn render_event_log(frame: &mut Frame<'_>, state: &MonitorState, area: Rect) {
    let log_lines: Vec<Line> = state
        .log
        .iter()
        .map(|entry| {
            Line::from(Span::raw(format!(
                " {}  {}",
                entry.timestamp, entry.message
            )))
        })
        .collect();

    let list = List::new(log_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " Event Log ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(list, area);
}

fn health_label(health: &crate::deploy::health::HealthStatus) -> String {
    match health {
        crate::deploy::health::HealthStatus::Healthy => "Healthy".to_string(),
        crate::deploy::health::HealthStatus::Degraded(d) => format!("Degraded ({})", d),
        crate::deploy::health::HealthStatus::Offline(d) => format!("Offline ({})", d),
    }
}
