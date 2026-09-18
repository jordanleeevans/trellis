use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::stack::{Layer, LayerDetail, StackSummary};
use crate::theme::glyphs::{GlyphSet, NERD_FONT};
use crate::tui::app::{Action, AppState, Component, Screen, layer_detail_cache_key};

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

    pub fn selected_index(&self) -> Option<usize> {
        self.list_state.selected()
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
    render_stack(frame, content_area, state, stack, list_state);
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
        Span::styled("j/k", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" navigate  "),
        Span::styled("O", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" open PR  "),
        Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" refresh detail  "),
        Span::styled("esc/q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" back"),
    ]);

    frame.render_widget(Paragraph::new(content), area);
}

fn render_stack(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    stack: &StackSummary,
    list_state: &mut ListState,
) {
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
        .highlight_symbol(format!("{} ", glyphs().current));

    frame.render_stateful_widget(list, list_area, list_state);

    render_layer_detail(frame, detail_area, state, stack, list_state.selected());
}

fn render_layer_detail(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
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

    let detail = state
        .layer_detail_cache
        .get(&layer_detail_cache_key(stack, layer));
    let lines = detail_lines(layer, rebase_status, detail);

    frame.render_widget(
        Paragraph::new(lines)
            .block(detail_block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn detail_lines(
    layer: &Layer,
    rebase_status: Span<'static>,
    detail: Option<&LayerDetail>,
) -> Vec<Line<'static>> {
    let pr = layer.pull_request.as_ref();
    let mut lines = vec![
        labeled_line("Branch", layer.branch.clone()),
        Line::from(vec![label_span("Status"), rebase_status]),
        labeled_line("Base", layer.base.clone()),
        labeled_line(
            "Head",
            layer.head.clone().unwrap_or_else(|| "Unknown".to_string()),
        ),
        labeled_line(
            "PR",
            pr.map(|pr| format!("#{} {}", pr.number, pr.state))
                .map(|text| format!("{} {text}", glyphs().pull_request))
                .unwrap_or_else(|| "Not submitted".to_string()),
        ),
    ];

    if let Some(pr) = pr {
        lines.push(Line::from(vec![
            label_span("URL"),
            Span::styled(
                pr.url.clone(),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]));
    }

    let Some(detail) = detail else {
        if pr.is_some() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Detail not loaded yet.",
                Style::default().fg(Color::DarkGray),
            )));
        }
        return lines;
    };

    lines.push(Line::from(""));
    lines.push(labeled_line("Title", detail.pull_request.title.clone()));

    if let Some(snippet) = &detail.pull_request.description_snippet {
        lines.push(labeled_line("Summary", snippet.clone()));
    }

    lines.push(labeled_line(
        "Labels",
        list_or_dash(&detail.pull_request.labels),
    ));
    lines.push(labeled_line(
        "Reviewers",
        reviewers_text(&detail.pull_request.reviewers),
    ));

    let checks = detail.pull_request.checks;
    lines.push(labeled_line(
        "Checks",
        format!(
            "{} total, {} pass, {} fail, {} pending",
            checks.total, checks.passing, checks.failing, checks.pending
        ),
    ));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Commits",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )));

    if detail.commits.is_empty() {
        lines.push(Line::from("  -"));
    } else {
        for commit in &detail.commits {
            let short_hash = commit.oid.chars().take(7).collect::<String>();
            let author = commit.author.as_deref().unwrap_or("unknown");
            lines.push(Line::from(format!(
                "  {short_hash}  {}  {author}  {}",
                commit.subject, commit.authored_at
            )));
        }
    }

    lines
}

fn labeled_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![label_span(label), Span::raw(value)])
}

fn label_span(label: &str) -> Span<'static> {
    Span::styled(
        format!("{label:<10}"),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
}

fn list_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(", ")
    }
}

fn reviewers_text(reviewers: &[crate::stack::ReviewerState]) -> String {
    if reviewers.is_empty() {
        return "-".to_string();
    }

    reviewers
        .iter()
        .map(|reviewer| format!("{} {}", reviewer.login, reviewer.state))
        .collect::<Vec<_>>()
        .join(", ")
}

fn glyphs() -> &'static GlyphSet {
    &NERD_FONT
}

fn row(layer: &Layer) -> Line<'static> {
    let marker = if layer.is_current {
        format!("{} ", glyphs().current)
    } else {
        "  ".to_string()
    };

    let mut style = Style::default();

    if layer.is_current {
        style = style.fg(Color::LightGreen).add_modifier(Modifier::BOLD);
    }

    let status = match &layer.pull_request {
        Some(pr) if layer.is_merged => {
            format!("{} #{} merged", glyphs().pull_request_merged, pr.number)
        }
        Some(pr) if pr.is_draft == Some(true) => {
            format!("{} #{} draft", glyphs().pull_request_open, pr.number)
        }
        Some(pr) => {
            format!(
                "{} #{} {}",
                glyphs().pull_request_open,
                pr.number,
                pr.state.to_lowercase()
            )
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
            KeyCode::Char('r') => self
                .list_state
                .selected()
                .map(|layer_index| {
                    vec![Action::LoadLayerDetail {
                        stack_index,
                        layer_index,
                        force: true,
                    }]
                })
                .unwrap_or_default(),
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
    use crate::stack::{
        CheckSummary, LayerCommit, LayerDetail, PullRequestDetail, PullRequestRef, ReviewerState,
    };
    use crate::test_fixtures::stack_summary;

    fn app_state(stacks: Vec<StackSummary>, screen: Screen) -> AppState {
        AppState {
            stacks,
            screen,
            status: None,
            layer_detail_cache: Default::default(),
            should_quit: false,
        }
    }

    #[test]
    fn show_layers_preserves_selection_for_same_stack() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack_summary("a", 3)], Screen::Layers(0));

        component.update(&Action::ShowLayers(0), &mut state);
        component.update(&Action::SelectNext, &mut state);
        component.update(&Action::ShowLayers(0), &mut state);

        assert_eq!(component.list_state.selected(), Some(1));
    }

    #[test]
    fn show_layers_resets_selection_when_switching_stacks() {
        let mut component = StackLayers::new();
        let mut state = app_state(
            vec![stack_summary("a", 3), stack_summary("b", 3)],
            Screen::Layers(0),
        );

        component.update(&Action::ShowLayers(0), &mut state);
        component.update(&Action::SelectNext, &mut state);
        component.update(&Action::ShowLayers(1), &mut state);

        assert_eq!(component.list_state.selected(), Some(0));
    }

    #[test]
    fn stacks_loaded_preserves_selection_for_same_stack_label() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack_summary("a", 3)], Screen::Layers(0));

        component.update(&Action::ShowLayers(0), &mut state);
        component.update(&Action::SelectNext, &mut state);
        state.stacks = vec![stack_summary("x", 1), stack_summary("a", 3)];
        state.screen = Screen::Layers(1);
        component.update(&Action::StacksLoaded(Some(1)), &mut state);

        assert_eq!(component.list_state.selected(), Some(1));
    }

    #[test]
    fn stacks_loaded_resets_selection_for_different_stack_label() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack_summary("a", 3)], Screen::Layers(0));

        component.update(&Action::ShowLayers(0), &mut state);
        component.update(&Action::SelectNext, &mut state);
        state.stacks = vec![stack_summary("b", 3)];
        state.screen = Screen::Layers(0);
        component.update(&Action::StacksLoaded(Some(0)), &mut state);

        assert_eq!(component.list_state.selected(), Some(0));
    }

    #[test]
    fn handle_key_maps_navigation_actions() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack_summary("a", 3)], Screen::Layers(0));
        component.update(&Action::ShowLayers(0), &mut state);

        let quit = component.handle_key(KeyCode::Char('q'), &state);
        let next = component.handle_key(KeyCode::Down, &state);
        let previous = component.handle_key(KeyCode::Up, &state);
        let refresh = component.handle_key(KeyCode::Char('r'), &state);

        assert!(matches!(quit.as_slice(), [Action::ShowList]));
        assert!(matches!(next.as_slice(), [Action::SelectNext]));
        assert!(matches!(previous.as_slice(), [Action::SelectPrevious]));
        assert!(matches!(
            refresh.as_slice(),
            [Action::LoadLayerDetail {
                stack_index: 0,
                layer_index: 0,
                force: true,
            }]
        ));
    }

    #[test]
    fn handle_key_dispatches_open_pr_for_selected_layer() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack_summary("a", 3)], Screen::Layers(0));
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

    #[test]
    fn detail_lines_include_cached_layer_detail() {
        let mut layer = crate::test_fixtures::layer("feature/layer-1");
        layer.pull_request = Some(PullRequestRef {
            number: 42,
            url: "https://example.test/pull/42".to_string(),
            state: "OPEN".to_string(),
            title: None,
            is_draft: None,
            checks_status: None,
            review_decision: None,
        });
        let detail = LayerDetail {
            commits: vec![LayerCommit {
                oid: "abcdef123456".to_string(),
                subject: "feat: render details".to_string(),
                author: Some("john-doe".to_string()),
                authored_at: "2026-09-18T10:00:00Z".to_string(),
            }],
            pull_request: PullRequestDetail {
                title: "Layer detail pane".to_string(),
                description_snippet: Some("Shows the selected layer.".to_string()),
                reviewers: vec![ReviewerState {
                    login: "octocat".to_string(),
                    state: "APPROVED".to_string(),
                }],
                checks: CheckSummary {
                    total: 2,
                    passing: 1,
                    failing: 0,
                    pending: 1,
                },
                labels: vec!["tui".to_string()],
            },
        };

        let text = detail_lines(&layer, Span::raw("Up to date"), Some(&detail))
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(text.contains("Layer detail pane"));
        assert!(text.contains("Shows the selected layer."));
        assert!(text.contains("tui"));
        assert!(text.contains("octocat APPROVED"));
        assert!(text.contains("2 total, 1 pass, 0 fail, 1 pending"));
        assert!(text.contains("abcdef1  feat: render details  john-doe"));
    }
}
