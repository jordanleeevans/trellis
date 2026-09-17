use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::stack::{Layer, StackSummary};
use crate::tui::app::{Action, AppState, Component, Screen};

pub struct StackLayers {
    list_state: ListState,
    active_stack_label: Option<String>,
}

impl StackLayers {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            active_stack_label: None,
        }
    }
}

fn render(frame: &mut Frame, state: &AppState, list_state: &mut ListState, index: usize) {
    let [header_area, content_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    let Some(stack) = state.stacks.get(index) else {
        frame.render_widget(Paragraph::new("Stack no longer available."), content_area);
        return;
    };

    render_header(frame, header_area, stack);
    render_stack(frame, content_area, stack, list_state);
    render_footer(frame, footer_area);
}

fn render_header(frame: &mut Frame, area: Rect, stack: &StackSummary) {
    let header = Block::default()
        .title(Span::styled(
            " stack details ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    frame.render_widget(
        Paragraph::new(format!("{} (trunk: {})", stack.label, stack.trunk))
            .style(
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            )
            .block(header),
        area,
    );
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let content = Line::from(vec![
        Span::styled(
            "j/k",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" navigate  "),
        Span::styled(
            "O",
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" open PR  "),
        Span::styled(
            "esc/q",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" back"),
    ]);

    frame.render_widget(
        Paragraph::new(content)
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
        area,
    );
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
        .block(
            Block::default()
                .title(Span::styled(
                    " layers ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

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
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .title(Span::styled(
                            " details ",
                            Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD),
                        ))
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Magenta)),
                ),
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

    let detail_block = Block::default()
        .title(Span::styled(
            " details ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta));

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
        style = style.fg(Color::LightGreen).add_modifier(Modifier::BOLD);
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

impl Component for StackLayers {
    fn draw(&mut self, frame: &mut Frame, state: &AppState) {
        if let Screen::Layers(index) = state.screen {
            render(frame, state, &mut self.list_state, index);
        }
    }

    fn handle_key(&mut self, code: KeyCode, state: &AppState) -> Vec<Action> {
        let Screen::Layers(stack_index) = state.screen else {
            return Vec::new();
        };

        match code {
            KeyCode::Char('q') | KeyCode::Esc => vec![Action::ShowList],
            KeyCode::Down | KeyCode::Char('j') => vec![Action::SelectNext],
            KeyCode::Up | KeyCode::Char('k') => vec![Action::SelectPrevious],
            KeyCode::Char('O') => self
                .list_state
                .selected()
                .map(|layer_index| {
                    vec![Action::OpenPullRequest {
                        stack_index,
                        layer_index,
                    }]
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn update(&mut self, action: &Action, state: &mut AppState) {
        match action {
            Action::ShowLayers(stack_index) => {
                let Some(stack) = state.stacks.get(*stack_index) else {
                    return;
                };
                let count = stack.layers.len();
                let stack_label = Some(stack.label.clone());
                let preserve_selection =
                    self.active_stack_label.as_deref() == stack_label.as_deref();
                let next_selection =
                    clamped_selection(count, self.list_state.selected(), preserve_selection);
                self.list_state.select(next_selection);
                self.active_stack_label = stack_label;
            }
            Action::SelectNext if matches!(state.screen, Screen::Layers(_)) => {
                select_next(&mut self.list_state, active_layer_count(state));
            }
            Action::SelectPrevious if matches!(state.screen, Screen::Layers(_)) => {
                select_previous(&mut self.list_state, active_layer_count(state));
            }
            Action::StacksLoaded(_) => {
                let active_stack = if let Screen::Layers(stack_index) = state.screen {
                    state.stacks.get(stack_index)
                } else {
                    None
                };
                let layer_count = active_stack.map(|stack| stack.layers.len()).unwrap_or(0);
                let active_label = active_stack.map(|stack| stack.label.clone());
                let preserve_selection =
                    self.active_stack_label.as_deref() == active_label.as_deref();
                let next_selection =
                    clamped_selection(layer_count, self.list_state.selected(), preserve_selection);
                self.list_state.select(next_selection);
                self.active_stack_label = active_label;
            }
            _ => {}
        }
    }
}

fn active_layer_count(state: &AppState) -> usize {
    if let Screen::Layers(stack_index) = state.screen {
        state
            .stacks
            .get(stack_index)
            .map(|stack| stack.layers.len())
            .unwrap_or(0)
    } else {
        0
    }
}

fn clamped_selection(
    count: usize,
    selected: Option<usize>,
    preserve_selection: bool,
) -> Option<usize> {
    if count == 0 {
        None
    } else if preserve_selection {
        match selected {
            Some(index) if index < count => Some(index),
            _ => Some(0),
        }
    } else {
        Some(0)
    }
}

fn select_next(state: &mut ListState, count: usize) {
    if count == 0 {
        state.select(None);
        return;
    }

    let next = match state.selected() {
        Some(index) if index + 1 < count => index + 1,
        _ => 0,
    };

    state.select(Some(next));
}

fn select_previous(state: &mut ListState, count: usize) {
    if count == 0 {
        state.select(None);
        return;
    }

    let previous = match state.selected() {
        Some(0) | None => count - 1,
        Some(index) => index - 1,
    };

    state.select(Some(previous))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(name: &str) -> Layer {
        Layer {
            branch: name.to_string(),
            head: None,
            base: "main".to_string(),
            is_current: false,
            is_merged: false,
            is_queued: false,
            needs_rebase: false,
            pull_request: None,
            commits: Vec::new(),
            position: 0,
        }
    }

    fn stack(label: &str, layer_count: usize) -> StackSummary {
        StackSummary {
            label: label.to_string(),
            trunk: "main".to_string(),
            layers: (0..layer_count)
                .map(|index| layer(&format!("{label}-layer-{index}")))
                .collect(),
            is_current: false,
        }
    }

    fn app_state(stacks: Vec<StackSummary>, screen: Screen) -> AppState {
        AppState {
            stacks,
            screen,
            status: None,
            should_quit: false,
        }
    }

    #[test]
    fn show_layers_preserves_selection_for_same_stack() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack("a", 3)], Screen::Layers(0));

        component.update(&Action::ShowLayers(0), &mut state);
        component.update(&Action::SelectNext, &mut state);
        component.update(&Action::ShowLayers(0), &mut state);

        assert_eq!(component.list_state.selected(), Some(1));
    }

    #[test]
    fn show_layers_resets_selection_when_switching_stacks() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack("a", 3), stack("b", 3)], Screen::Layers(0));

        component.update(&Action::ShowLayers(0), &mut state);
        component.update(&Action::SelectNext, &mut state);
        component.update(&Action::ShowLayers(1), &mut state);

        assert_eq!(component.list_state.selected(), Some(0));
    }

    #[test]
    fn stacks_loaded_preserves_selection_for_same_stack_label() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack("a", 3)], Screen::Layers(0));

        component.update(&Action::ShowLayers(0), &mut state);
        component.update(&Action::SelectNext, &mut state);
        state.stacks = vec![stack("x", 1), stack("a", 3)];
        state.screen = Screen::Layers(1);
        component.update(&Action::StacksLoaded(Some(1)), &mut state);

        assert_eq!(component.list_state.selected(), Some(1));
    }

    #[test]
    fn stacks_loaded_resets_selection_for_different_stack_label() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack("a", 3)], Screen::Layers(0));

        component.update(&Action::ShowLayers(0), &mut state);
        component.update(&Action::SelectNext, &mut state);
        state.stacks = vec![stack("b", 3)];
        state.screen = Screen::Layers(0);
        component.update(&Action::StacksLoaded(Some(0)), &mut state);

        assert_eq!(component.list_state.selected(), Some(0));
    }

    #[test]
    fn handle_key_maps_navigation_actions() {
        let mut component = StackLayers::new();
        let state = app_state(vec![stack("a", 3)], Screen::Layers(0));

        let quit = component.handle_key(KeyCode::Char('q'), &state);
        let next = component.handle_key(KeyCode::Down, &state);
        let previous = component.handle_key(KeyCode::Up, &state);

        assert!(matches!(quit.as_slice(), [Action::ShowList]));
        assert!(matches!(next.as_slice(), [Action::SelectNext]));
        assert!(matches!(previous.as_slice(), [Action::SelectPrevious]));
    }

    #[test]
    fn handle_key_dispatches_open_pr_for_selected_layer() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack("a", 3)], Screen::Layers(0));
        component.update(&Action::ShowLayers(0), &mut state);
        component.update(&Action::SelectNext, &mut state);

        let actions = component.handle_key(KeyCode::Char('O'), &state);
        assert!(matches!(
            actions.as_slice(),
            [Action::OpenPullRequest {
                stack_index: 0,
                layer_index: 1
            }]
        ));
    }
}
