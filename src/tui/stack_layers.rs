use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::stack::{Layer, StackSummary};
use crate::tui::app::App;

pub fn render(frame: &mut Frame, app: &mut App, index: usize) {
    let [header_area, content_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let Some(stack) = app.stacks.get(index) else {
        frame.render_widget(Paragraph::new("Stack no longer available."), content_area);
        return;
    };

    render_header(frame, header_area, stack);
    render_stack(frame, content_area, stack, &mut app.layer_list_state);
    render_footer(frame, footer_area);
}

fn render_header(frame: &mut Frame, area: Rect, stack: &StackSummary) {
    frame.render_widget(
        Paragraph::new(format!("{} (trunk: {})", stack.label, stack.trunk))
            .style(Style::default().add_modifier(Modifier::BOLD)),
        area,
    );
}

fn render_footer(frame: &mut Frame, area: Rect) {
    frame.render_widget(Paragraph::new("esc/q back"), area);
}

fn render_stack(frame: &mut Frame, area: Rect, stack: &StackSummary, list_state: &mut ListState) {
    let [list_area, detail_area] = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .areas(area);

    let items: Vec<ListItem> = stack
        .layers
        .iter()
        .map(|layer| ListItem::new(row(layer)))
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Layers").borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, list_area, list_state);

    render_layer_detail(frame, detail_area, stack, list_state.selected());
}

fn render_layer_detail(
    frame: &mut Frame,
    area: Rect,
    stack: &StackSummary,
    selected: Option<usize>,
) {
    let Some(selected) = selected else {
        frame.render_widget(
            Paragraph::new("No layer selected")
                .block(Block::default().title("Details").borders(Borders::ALL)),
            area,
        );
        return;
    };

    let Some(layer) = stack.layers.get(selected) else {
        return;
    };

    let pr = layer.pull_request.as_ref();

    let rebase_status = if layer.needs_rebase {
        Span::styled(
            "Needs rebase",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "Up to date",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    };

    let detail_block = Block::default().title("Details").borders(Borders::ALL);

    let inner = detail_block.inner(area);

    frame.render_widget(detail_block, area);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(inner);

    render_detail_row(
        frame,
        rows[0],
        "Branch",
        Paragraph::new(layer.branch.as_str()).wrap(Wrap { trim: false }),
    );

    render_detail_row(
        frame,
        rows[1],
        "Status",
        Paragraph::new(Line::from(rebase_status)),
    );

    render_detail_row(
        frame,
        rows[2],
        "Base",
        Paragraph::new(layer.base.as_str()).wrap(Wrap { trim: false }),
    );

    render_detail_row(
        frame,
        rows[3],
        "Head",
        Paragraph::new(layer.head.as_deref().unwrap_or("Unknown")).wrap(Wrap { trim: false }),
    );

    let pr_content = vec![
        Line::from(
            pr.map(|pr| format!("#{} {}", pr.number, pr.state))
                .unwrap_or_else(|| "Not submitted".to_string()),
        ),
        Line::from(Span::styled(
            pr.map(|pr| pr.url.as_str()).unwrap_or("Not submitted"),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED),
        )),
    ];

    render_detail_row(
        frame,
        rows[4],
        "PR",
        Paragraph::new(pr_content).wrap(Wrap { trim: false }),
    );
}

fn render_detail_row<'a>(frame: &mut Frame, area: Rect, label: &'a str, value: Paragraph<'a>) {
    let [label_area, value_area] =
        Layout::horizontal([Constraint::Length(10), Constraint::Min(0)]).areas(area);

    frame.render_widget(
        Paragraph::new(label).style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        label_area,
    );

    frame.render_widget(value, value_area);
}

fn row(layer: &Layer) -> Line<'static> {
    let marker = if layer.is_current { "* " } else { "  " };

    let mut style = Style::default();

    if layer.is_current {
        style = style.fg(Color::Green).add_modifier(Modifier::BOLD);
    }

    let status = match &layer.pull_request {
        Some(pr) if layer.is_merged => {
            format!("#{} merged", pr.number)
        }
        Some(pr) if pr.is_draft == Some(true) => {
            format!("#{} draft", pr.number)
        }
        Some(pr) => {
            format!("#{} {}", pr.number, pr.state.to_lowercase())
        }
        None => "not submitted".to_string(),
    };

    let text = format!(
        "{marker}{branch} (base: {base})  {status}",
        branch = layer.branch,
        base = layer.base,
    );

    Line::from(Span::styled(text, style))
}
