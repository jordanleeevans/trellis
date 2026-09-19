use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::stack::{Layer, LayerDetail, StackSummary};
use crate::theme::glyphs::{GlyphSet, NERD_FONT};
use crate::theme::ui::THEME;
use crate::tui::app::{
    Action, AppState, Component, Screen, layer_detail_cache_key, layer_diff_cache_key,
    lower_layer_ref, spinner_frame,
};

use super::keymap::{KeyIntent, key_intent};
use super::panel::panel_block;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivePanel {
    Layers,
    Detail,
    Files,
    Diff,
}

pub struct StackLayers {
    list_state: ListState,
    active_stack_label: Option<String>,
    active_panel: ActivePanel,
    selected_diff_file: usize,
    diff_scroll: u16,
    active_layer_key: Option<String>,
}

impl StackLayers {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            active_stack_label: None,
            active_panel: ActivePanel::Layers,
            selected_diff_file: 0,
            diff_scroll: 0,
            active_layer_key: None,
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.list_state.selected()
    }
}

fn render(
    frame: &mut Frame,
    state: &AppState,
    list_state: &mut ListState,
    index: usize,
    active_panel: ActivePanel,
    selected_diff_file: usize,
    diff_scroll: u16,
) {
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
    render_stack(
        frame,
        content_area,
        state,
        stack,
        list_state,
        active_panel,
        selected_diff_file,
        diff_scroll,
    );
    render_footer(frame, footer_area, state);
}

fn render_header(frame: &mut Frame, area: Rect, stack: &StackSummary) {
    let header = Block::default()
        .title(Span::styled(" stack details ", THEME.text.heading))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(THEME.tertiary_border());

    frame.render_widget(
        Paragraph::new(format!("{} (trunk: {})", stack.label, stack.trunk))
            .style(
                Style::default()
                    .fg(THEME.colors.secondary)
                    .add_modifier(Modifier::BOLD),
            )
            .block(header),
        area,
    );
}

fn render_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let content = footer_line(state);

    frame.render_widget(Paragraph::new(content), area);
}

fn footer_line(state: &AppState) -> Line<'static> {
    if let Some(error) = &state.error {
        Line::from(vec![
            Span::styled("error: ", THEME.text.key.fg(THEME.colors.danger)),
            Span::raw(error.clone()),
            Span::raw("  "),
            Span::styled("x", THEME.text.key.fg(THEME.colors.warning)),
            Span::raw(" dismiss"),
        ])
    } else if state.refresh_in_flight {
        Line::from(vec![
            Span::styled(
                format!("{} ", spinner_frame(state.refresh_spinner_frame)),
                THEME.text.key.fg(THEME.colors.secondary),
            ),
            Span::raw("Refreshing stacks in background"),
        ])
    } else {
        Line::from(vec![
            Span::styled("j/k", THEME.text.key),
            Span::raw(" navigate  "),
            Span::styled("O", THEME.text.key),
            Span::raw(" open PR  "),
            Span::styled("r", THEME.text.key),
            Span::raw(" refresh  "),
            Span::styled("tab/h/l", THEME.text.key),
            Span::raw(" panels  "),
            Span::styled("PgUp/PgDn", THEME.text.key),
            Span::raw(" diff  "),
            Span::styled("esc/q", THEME.text.key),
            Span::raw(" back"),
        ])
    }
}

fn render_stack(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    stack: &StackSummary,
    list_state: &mut ListState,
    active_panel: ActivePanel,
    selected_diff_file: usize,
    diff_scroll: u16,
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
        .block(panel_block("layers", active_panel == ActivePanel::Layers))
        .highlight_style(THEME.text.selected)
        .highlight_symbol(format!("{} ", glyphs().current));

    frame.render_stateful_widget(list, list_area, list_state);

    render_layer_detail(
        frame,
        detail_area,
        state,
        stack,
        list_state.selected(),
        active_panel,
        selected_diff_file,
        diff_scroll,
    );
}

fn render_layer_detail(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    stack: &StackSummary,
    selected: Option<usize>,
    active_panel: ActivePanel,
    selected_diff_file: usize,
    diff_scroll: u16,
) {
    let Some(selected) = selected else {
        frame.render_widget(
            Paragraph::new("No layer selected")
                .style(THEME.text.muted)
                .block(panel_block("details", active_panel == ActivePanel::Detail)),
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
            Style::default()
                .fg(THEME.colors.danger)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "Up to date",
            Style::default()
                .fg(THEME.colors.success)
                .add_modifier(Modifier::BOLD),
        )
    };

    let detail_block = panel_block("details", active_panel == ActivePanel::Detail);

    let detail = state
        .layer_details
        .get(&layer_detail_cache_key(stack, layer));
    let lines = detail_lines(layer, rebase_status, detail);

    let [summary_area, files_area, diff_area] = Layout::vertical([
        Constraint::Length(9),
        Constraint::Length(6),
        Constraint::Min(0),
    ])
    .areas(area);

    frame.render_widget(
        Paragraph::new(lines)
            .block(detail_block)
            .wrap(Wrap { trim: false }),
        summary_area,
    );

    let diff_key = layer_diff_cache_key(stack, layer);
    let diff = state.layer_diffs.get(&diff_key).map(String::as_str);
    let diff_loading = state.layer_diffs.is_loading(&diff_key);
    render_diff_files(
        frame,
        files_area,
        diff,
        diff_loading,
        active_panel == ActivePanel::Files,
        selected_diff_file,
    );
    render_diff(
        frame,
        diff_area,
        stack,
        selected,
        diff,
        diff_loading,
        active_panel == ActivePanel::Diff,
        selected_diff_file,
        diff_scroll,
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiffFile {
    path: String,
    start: usize,
    end: usize,
}

fn parse_diff_files(diff: &str) -> Vec<DiffFile> {
    let lines: Vec<&str> = diff.lines().collect();
    let mut files: Vec<DiffFile> = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if let Some(path) = line.strip_prefix("diff --git a/") {
            if let Some(previous) = files.last_mut() {
                previous.end = index;
            }

            let path = path.split(" b/").nth(1).unwrap_or(path).to_string();
            files.push(DiffFile {
                path,
                start: index,
                end: lines.len(),
            });
        }
    }

    files
}

fn render_diff_files(
    frame: &mut Frame,
    area: Rect,
    diff: Option<&str>,
    is_loading: bool,
    is_active: bool,
    selected_diff_file: usize,
) {
    let block = panel_block("files", is_active);

    let Some(diff) = diff else {
        let message = if is_loading {
            "Loading diff..."
        } else {
            "Diff not loaded."
        };
        frame.render_widget(
            Paragraph::new(message).style(THEME.text.muted).block(block),
            area,
        );
        return;
    };

    let files = parse_diff_files(diff);
    if files.is_empty() {
        frame.render_widget(
            Paragraph::new("No changes in this layer.")
                .style(THEME.text.muted)
                .block(block),
            area,
        );
        return;
    }

    let visible_rows = area.height.saturating_sub(2) as usize;
    let visible_range = visible_file_range(selected_diff_file, files.len(), visible_rows);
    let visible_start = visible_range.start;
    let items = files[visible_range]
        .iter()
        .enumerate()
        .map(|(offset, file)| (visible_start + offset, file))
        .map(|(index, file)| {
            let marker = if index == selected_diff_file {
                ">"
            } else {
                " "
            };
            ListItem::new(Line::from(format!("{marker} {}", file.path)))
        })
        .collect::<Vec<_>>();

    frame.render_widget(List::new(items).block(block), area);
}

fn visible_file_range(
    selected_index: usize,
    file_count: usize,
    visible_rows: usize,
) -> std::ops::Range<usize> {
    if file_count == 0 || visible_rows == 0 {
        return 0..0;
    }

    let selected_index = selected_index.min(file_count - 1);
    let visible_rows = visible_rows.min(file_count);
    let half_window = visible_rows / 2;
    let mut start = selected_index.saturating_sub(half_window);

    if start + visible_rows > file_count {
        start = file_count - visible_rows;
    }

    start..start + visible_rows
}

fn render_diff(
    frame: &mut Frame,
    area: Rect,
    stack: &StackSummary,
    selected_layer: usize,
    diff: Option<&str>,
    is_loading: bool,
    is_active: bool,
    selected_diff_file: usize,
    diff_scroll: u16,
) {
    let Some(layer) = stack.layers.get(selected_layer) else {
        return;
    };
    let title = format!(
        " diff {}..{} ",
        lower_layer_ref(stack, selected_layer),
        layer.branch
    );
    let block = panel_block(title, is_active);

    let Some(diff) = diff else {
        let message = if is_loading {
            "Loading diff..."
        } else {
            "Diff unavailable."
        };
        frame.render_widget(
            Paragraph::new(message).style(THEME.text.muted).block(block),
            area,
        );
        return;
    };

    let files = parse_diff_files(diff);
    if files.is_empty() {
        frame.render_widget(Paragraph::new("No changes.").block(block), area);
        return;
    }

    let lines = diff.lines().collect::<Vec<_>>();
    let file = &files[selected_diff_file.min(files.len() - 1)];
    let visible_height = area.height.saturating_sub(2) as usize;
    let start = (file.start + usize::from(diff_scroll)).min(file.end);
    let end = (start + visible_height).min(file.end);
    let rendered = lines[start..end]
        .iter()
        .map(|line| diff_line(line))
        .collect::<Vec<_>>();

    frame.render_widget(Paragraph::new(rendered).block(block), area);
}

fn diff_line(text: &str) -> Line<'static> {
    let style = if text.starts_with("+++") || text.starts_with("---") {
        Style::default().fg(THEME.colors.text_muted)
    } else if text.starts_with('+') {
        Style::default().fg(THEME.colors.success)
    } else if text.starts_with('-') {
        Style::default().fg(THEME.colors.danger)
    } else if text.starts_with("@@") {
        Style::default()
            .fg(THEME.colors.primary)
            .add_modifier(Modifier::BOLD)
    } else if text.starts_with("diff --git") {
        Style::default()
            .fg(THEME.colors.secondary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    Line::from(Span::styled(text.to_string(), style))
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
                    .fg(THEME.colors.link)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]));
    }

    let Some(detail) = detail else {
        if pr.is_some() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Detail not loaded yet.",
                THEME.text.muted,
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
            .fg(THEME.colors.text_muted)
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
    Span::styled(format!("{label:<10}"), THEME.text.label)
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
        style = style.fg(THEME.colors.success).add_modifier(Modifier::BOLD);
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
            render(
                frame,
                state,
                &mut self.list_state,
                index,
                self.active_panel,
                self.selected_diff_file,
                self.diff_scroll,
            );
        }
    }

    fn handle_key(&mut self, code: KeyCode, state: &AppState) -> Vec<Action> {
        let Screen::Layers(stack_index) = state.screen else {
            return Vec::new();
        };

        match key_intent(code) {
            Some(KeyIntent::Back) => vec![Action::ShowList],
            Some(KeyIntent::FocusNext) => vec![Action::FocusNextPanel],
            Some(KeyIntent::FocusPrevious) => vec![Action::FocusPreviousPanel],
            Some(KeyIntent::MoveDown) => match self.active_panel {
                ActivePanel::Layers => vec![Action::SelectNext],
                ActivePanel::Files => vec![Action::SelectNextDiffFile],
                ActivePanel::Diff => vec![Action::ScrollDiffLineDown],
                ActivePanel::Detail => Vec::new(),
            },
            Some(KeyIntent::MoveUp) => match self.active_panel {
                ActivePanel::Layers => vec![Action::SelectPrevious],
                ActivePanel::Files => vec![Action::SelectPreviousDiffFile],
                ActivePanel::Diff => vec![Action::ScrollDiffLineUp],
                ActivePanel::Detail => Vec::new(),
            },
            Some(KeyIntent::PageDown) => vec![Action::ScrollDiffDown],
            Some(KeyIntent::PageUp) => vec![Action::ScrollDiffUp],
            Some(KeyIntent::Refresh) => self
                .list_state
                .selected()
                .map(|layer_index| {
                    vec![
                        Action::LoadLayerDetail {
                            stack_index,
                            layer_index,
                            force: true,
                        },
                        Action::LoadLayerDiff {
                            stack_index,
                            layer_index,
                            force: true,
                        },
                    ]
                })
                .unwrap_or_default(),
            Some(KeyIntent::DrillIn) | Some(KeyIntent::OpenExternal) => self
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
                self.reset_diff_view_for_selection(state);
            }
            Action::SelectNext if matches!(state.screen, Screen::Layers(_)) => {
                select_next(&mut self.list_state, active_layer_count(state));
                self.reset_diff_view_for_selection(state);
            }
            Action::SelectPrevious if matches!(state.screen, Screen::Layers(_)) => {
                select_previous(&mut self.list_state, active_layer_count(state));
                self.reset_diff_view_for_selection(state);
            }
            Action::FocusNextPanel => {
                self.active_panel = next_panel(self.active_panel);
            }
            Action::FocusPreviousPanel => {
                self.active_panel = previous_panel(self.active_panel);
            }
            Action::SelectNextDiffFile => {
                self.select_next_diff_file(state);
            }
            Action::SelectPreviousDiffFile => {
                self.select_previous_diff_file(state);
            }
            Action::ScrollDiffDown => {
                self.scroll_diff_down();
            }
            Action::ScrollDiffUp => {
                self.scroll_diff_up();
            }
            Action::ScrollDiffLineDown => {
                self.scroll_diff_line_down();
            }
            Action::ScrollDiffLineUp => {
                self.scroll_diff_line_up();
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
                self.reset_diff_view_for_selection(state);
            }
            _ => {}
        }
    }
}

impl StackLayers {
    fn reset_diff_view_for_selection(&mut self, state: &AppState) {
        let next_key = active_layer_key(state, self.list_state.selected());
        if self.active_layer_key != next_key {
            self.selected_diff_file = 0;
            self.diff_scroll = 0;
            self.active_layer_key = next_key;
        }
    }

    fn select_next_diff_file(&mut self, state: &AppState) {
        let count = active_diff_file_count(state, self.list_state.selected());
        if count == 0 {
            self.selected_diff_file = 0;
            return;
        }
        self.selected_diff_file = (self.selected_diff_file + 1) % count;
        self.diff_scroll = 0;
    }

    fn select_previous_diff_file(&mut self, state: &AppState) {
        let count = active_diff_file_count(state, self.list_state.selected());
        if count == 0 {
            self.selected_diff_file = 0;
            return;
        }
        self.selected_diff_file = if self.selected_diff_file == 0 {
            count - 1
        } else {
            self.selected_diff_file - 1
        };
        self.diff_scroll = 0;
    }

    fn scroll_diff_down(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_add(12);
    }

    fn scroll_diff_up(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_sub(12);
    }

    fn scroll_diff_line_down(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_add(1);
    }

    fn scroll_diff_line_up(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_sub(1);
    }
}

fn next_panel(panel: ActivePanel) -> ActivePanel {
    match panel {
        ActivePanel::Layers => ActivePanel::Detail,
        ActivePanel::Detail => ActivePanel::Files,
        ActivePanel::Files => ActivePanel::Diff,
        ActivePanel::Diff => ActivePanel::Layers,
    }
}

fn previous_panel(panel: ActivePanel) -> ActivePanel {
    match panel {
        ActivePanel::Layers => ActivePanel::Diff,
        ActivePanel::Detail => ActivePanel::Layers,
        ActivePanel::Files => ActivePanel::Detail,
        ActivePanel::Diff => ActivePanel::Files,
    }
}

fn active_layer_key(state: &AppState, selected: Option<usize>) -> Option<String> {
    let Screen::Layers(stack_index) = state.screen else {
        return None;
    };
    let stack = state.stacks.get(stack_index)?;
    let layer = stack.layers.get(selected?)?;
    Some(layer_diff_cache_key(stack, layer))
}

fn active_diff_file_count(state: &AppState, selected: Option<usize>) -> usize {
    let Screen::Layers(stack_index) = state.screen else {
        return 0;
    };
    let Some(stack) = state.stacks.get(stack_index) else {
        return 0;
    };
    let Some(layer) = selected.and_then(|index| stack.layers.get(index)) else {
        return 0;
    };
    state
        .layer_diffs
        .get(&layer_diff_cache_key(stack, layer))
        .map(|diff| parse_diff_files(diff).len())
        .unwrap_or(0)
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
    use std::collections::HashMap;

    use super::*;
    use crate::stack::{
        CheckSummary, LayerCommit, LayerDetail, PullRequestDetail, PullRequestRef, ReviewerState,
    };
    use crate::test_fixtures::stack_summary;
    use crate::tui::layer_resource::LayerResourceCache;

    fn app_state(stacks: Vec<StackSummary>, screen: Screen) -> AppState {
        AppState {
            stacks,
            screen,
            status: None,
            error: None,
            refresh_in_flight: false,
            refresh_spinner_frame: 0,
            refresh_request_id: 0,
            refresh_active_request_id: None,
            last_successful_stacks: Vec::new(),
            layer_details: LayerResourceCache::default(),
            layer_diffs: LayerResourceCache::default(),
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
            [
                Action::LoadLayerDetail {
                    stack_index: 0,
                    layer_index: 0,
                    force: true,
                },
                Action::LoadLayerDiff {
                    stack_index: 0,
                    layer_index: 0,
                    force: true,
                }
            ]
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
    fn tab_focus_changes_what_list_movement_controls() {
        let mut component = StackLayers::new();
        let mut state = app_state(vec![stack_summary("a", 1)], Screen::Layers(0));
        component.update(&Action::ShowLayers(0), &mut state);

        let key = layer_diff_cache_key(&state.stacks[0], &state.stacks[0].layers[0]);
        state.layer_diffs.store_result(
            key,
            Ok([
                "diff --git a/src/a.rs b/src/a.rs",
                "+a",
                "diff --git a/src/b.rs b/src/b.rs",
                "+b",
            ]
            .join("\n")),
        );

        component.update(&Action::FocusNextPanel, &mut state);
        component.update(&Action::FocusNextPanel, &mut state);
        let file_next = component.handle_key(KeyCode::Char('j'), &state);
        assert!(matches!(file_next.as_slice(), [Action::SelectNextDiffFile]));
        component.update(&file_next[0], &mut state);
        assert_eq!(component.selected_diff_file, 1);

        component.update(&Action::FocusNextPanel, &mut state);
        let diff_down = component.handle_key(KeyCode::Down, &state);
        assert!(matches!(diff_down.as_slice(), [Action::ScrollDiffLineDown]));
        component.update(&diff_down[0], &mut state);
        assert_eq!(component.diff_scroll, 1);
    }

    #[test]
    fn parse_diff_files_finds_each_file_range() {
        let diff = [
            "diff --git a/src/a.rs b/src/a.rs",
            "index 123..456 100644",
            "--- a/src/a.rs",
            "+++ b/src/a.rs",
            "@@ -1 +1 @@",
            "-old",
            "+new",
            "diff --git a/src/b.rs b/src/b.rs",
            "@@ -3 +3 @@",
            "+more",
        ]
        .join("\n");

        let files = parse_diff_files(&diff);

        assert_eq!(
            files,
            vec![
                DiffFile {
                    path: "src/a.rs".to_string(),
                    start: 0,
                    end: 7,
                },
                DiffFile {
                    path: "src/b.rs".to_string(),
                    start: 7,
                    end: 10,
                },
            ]
        );
    }

    #[test]
    fn visible_file_range_keeps_early_selection_at_top() {
        assert_eq!(visible_file_range(1, 10, 4), 0..4);
    }

    #[test]
    fn visible_file_range_centers_middle_selection() {
        assert_eq!(visible_file_range(6, 12, 5), 4..9);
    }

    #[test]
    fn visible_file_range_keeps_late_selection_visible_at_bottom() {
        assert_eq!(visible_file_range(11, 12, 5), 7..12);
    }

    #[test]
    fn visible_file_range_handles_no_visible_rows() {
        assert_eq!(visible_file_range(3, 10, 0), 0..0);
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

    #[test]
    fn footer_shows_refresh_indicator_when_loading() {
        let mut state = app_state(vec![stack_summary("a", 1)], Screen::Layers(0));
        state.refresh_in_flight = true;
        let text = footer_line(&state)
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(text.contains("Refreshing stacks"));
    }

    #[test]
    fn footer_shows_dismissible_error() {
        let mut state = app_state(vec![stack_summary("a", 1)], Screen::Layers(0));
        state.error = Some("auth required".to_string());
        let text = footer_line(&state)
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(text.contains("error:"));
        assert!(text.contains("dismiss"));
    }
}
