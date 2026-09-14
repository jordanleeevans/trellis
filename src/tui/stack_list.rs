use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::stack::{PrCounts, StackSummary};

use super::app::App;

/// Renders the entry-point panel: every locally tracked stack.
pub fn render(frame: &mut ratatui::Frame, app: &App) {
    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_header(frame, header_area);
    render_list(frame, list_area, app);
    render_footer(frame, footer_area, app);
}

fn render_header(frame: &mut Frame, area: ratatui::layout::Rect) {
    frame.render_widget(
        Paragraph::new("Stacks").style(Style::default().add_modifier(Modifier::BOLD)),
        area,
    );
}

fn render_list(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    if app.stacks.is_empty() {
        frame.render_widget(
            Paragraph::new("No stacks found in this repository.")
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = app
        .stacks
        .iter()
        .map(|stack| ListItem::new(row(stack)))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = app.list_state;
    frame.render_stateful_widget(list, area, &mut state);
}

fn row(stack: &StackSummary) -> Line<'static> {
    let marker = if stack.is_current { "* " } else { "  " };

    let mut style = Style::default();
    if stack.is_current {
        style = style.fg(Color::Green).add_modifier(Modifier::BOLD);
    }

    let text = format!(
        "{marker}{label}  (trunk: {trunk}, {layers} layer{plural})  {status}",
        label = stack.label,
        trunk = stack.trunk,
        layers = stack.layer_count(),
        plural = if stack.layer_count() == 1 { "" } else { "s" },
        status = describe(&stack.pr_counts()),
    );

    Line::from(Span::styled(text, style))
}

fn describe(counts: &PrCounts) -> String {
    let mut parts = Vec::new();

    if counts.open > 0 {
        parts.push(format!("{} open", counts.open));
    }
    if counts.draft > 0 {
        parts.push(format!("{} draft", counts.draft));
    }
    if counts.merged > 0 {
        parts.push(format!("{} merged", counts.merged));
    }
    if counts.closed > 0 {
        parts.push(format!("{} closed", counts.closed));
    }
    if counts.unsubmitted > 0 {
        parts.push(format!("{} unsubmitted", counts.unsubmitted));
    }

    if parts.is_empty() {
        "no layers".to_string()
    } else {
        parts.join(", ")
    }
}

fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let text = app
        .status
        .clone()
        .unwrap_or_else(|| "↑/↓ select  enter view  r refresh  q quit".to_string());

    frame.render_widget(Paragraph::new(text), area);
}
