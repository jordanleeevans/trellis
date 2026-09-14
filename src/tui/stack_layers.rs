use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::stack::Layer;

use super::app::App;

/// Renders the layer view for the stack at `index`.
///
/// This is a minimal stub: it shows each layer's branch, base, and pull
/// request status. The full Stack Layer View (rebase state, commits,
/// navigation between layers) is a separate piece of work.
pub fn render(frame: &mut Frame, app: &App, index: usize) {
    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let Some(stack) = app.stacks.get(index) else {
        frame.render_widget(Paragraph::new("Stack no longer available."), list_area);
        return;
    };

    frame.render_widget(
        Paragraph::new(format!("{} (trunk: {})", stack.label, stack.trunk))
            .style(Style::default().add_modifier(Modifier::BOLD)),
        header_area,
    );

    let items: Vec<ListItem> = stack.layers.iter().map(|layer| ListItem::new(row(layer))).collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL));
    frame.render_widget(list, list_area);

    frame.render_widget(Paragraph::new("esc/q back"), footer_area);
}

fn row(layer: &Layer) -> Line<'static> {
    let marker = if layer.is_current { "* " } else { "  " };

    let mut style = Style::default();
    if layer.is_current {
        style = style.fg(Color::Green).add_modifier(Modifier::BOLD);
    }

    let status = match &layer.pull_request {
        Some(pr) if layer.is_merged => format!("#{} merged", pr.number),
        Some(pr) if pr.is_draft == Some(true) => format!("#{} draft", pr.number),
        Some(pr) => format!("#{} {}", pr.number, pr.state.to_lowercase()),
        None => "not submitted".to_string(),
    };

    let text = format!(
        "{marker}{branch} (base: {base})  {status}",
        branch = layer.branch,
        base = layer.base,
    );

    Line::from(Span::styled(text, style))
}
