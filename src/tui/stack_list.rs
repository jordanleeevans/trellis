use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListState;
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};

use crate::stack::{PrCounts, StackSummary};
use crate::theme::glyphs::{GlyphSet, NERD_FONT};
use crate::theme::ui::THEME;

use super::app::{Action, AppState, Component, Screen};
use super::keymap::{KeyIntent, key_intent};

pub struct StackList {
    list_state: ListState,
}

impl StackList {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.list_state.selected()
    }
}

/// Renders the entry-point panel: every locally tracked stack.
fn render(frame: &mut ratatui::Frame, state: &AppState, list_state: &mut ListState) {
    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    render_header(frame, header_area);
    render_list(frame, list_area, state, list_state);
    render_footer(frame, footer_area, state);
}

fn render_header(frame: &mut Frame, area: Rect) {
    let header = Block::default()
        .title(Line::from(vec![
            Span::styled(
                " Trellis ",
                Style::default()
                    .fg(THEME.colors.text_inverse)
                    .bg(THEME.colors.primary),
            ),
            Span::styled(" stacks", THEME.text.heading.fg(THEME.colors.secondary)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(THEME.primary_border());

    frame.render_widget(
        Paragraph::new("Browse locally tracked stacks and layer status")
            .style(THEME.text.body)
            .block(header),
        area,
    );
}

fn render_list(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    state: &AppState,
    list_state: &mut ListState,
) {
    if state.stacks.is_empty() {
        frame.render_widget(
            Paragraph::new("No stacks found in this repository.").block(
                Block::default()
                    .title(Span::styled(
                        " empty ",
                        THEME.text.heading.fg(THEME.colors.warning),
                    ))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(THEME.colors.warning)),
            ),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = state
        .stacks
        .iter()
        .map(|stack| ListItem::new(row(stack)))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(" tracked stacks ", THEME.text.heading))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(THEME.tertiary_border()),
        )
        .highlight_style(THEME.text.selected)
        .highlight_symbol(format!("{} ", glyphs().current));

    frame.render_stateful_widget(list, area, list_state);
}

fn row(stack: &StackSummary) -> Line<'static> {
    let marker = if stack.is_current {
        format!("{} ", glyphs().current)
    } else {
        "  ".to_string()
    };

    let mut style = Style::default();
    if stack.is_current {
        style = style.fg(THEME.colors.success).add_modifier(Modifier::BOLD);
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

fn render_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let text = state.status.clone().map(Line::from).unwrap_or_else(|| {
        Line::from(vec![
            Span::styled(format!("{}/{}", glyphs().up, glyphs().down), THEME.text.key),
            Span::raw(" select  "),
            Span::styled("enter", THEME.text.key.fg(THEME.colors.success)),
            Span::raw(" view  "),
            Span::styled("r", THEME.text.key.fg(THEME.colors.warning)),
            Span::raw(" refresh  "),
            Span::styled("q", THEME.text.key.fg(THEME.colors.danger)),
            Span::raw(" quit"),
        ])
    });

    frame.render_widget(
        Paragraph::new(text).style(THEME.text.body).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(THEME.colors.text_muted)),
        ),
        area,
    );
}

fn glyphs() -> &'static GlyphSet {
    &NERD_FONT
}

impl Component for StackList {
    fn draw(&mut self, frame: &mut Frame, state: &AppState) {
        render(frame, state, &mut self.list_state);
    }

    fn handle_key(&mut self, code: KeyCode, _state: &AppState) -> Vec<Action> {
        match key_intent(code) {
            Some(KeyIntent::Back) => vec![Action::Quit],
            Some(KeyIntent::Refresh) => vec![Action::RefreshStacks],
            Some(KeyIntent::MoveDown) => vec![Action::SelectNext],
            Some(KeyIntent::MoveUp) => vec![Action::SelectPrevious],
            Some(KeyIntent::DrillIn) => self
                .list_state
                .selected()
                .map(|index| vec![Action::ShowLayers(index)])
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn update(&mut self, action: &Action, state: &mut AppState) {
        match action {
            Action::SelectNext if matches!(state.screen, Screen::List) => {
                select_next(&mut self.list_state, state.stacks.len())
            }
            Action::SelectPrevious if matches!(state.screen, Screen::List) => {
                select_previous(&mut self.list_state, state.stacks.len())
            }
            Action::StacksLoaded(selected_index) => self.list_state.select(*selected_index),
            _ => {}
        }
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
    use crate::tui::app::Screen;

    fn app_state() -> AppState {
        AppState {
            stacks: vec![
                StackSummary {
                    label: "stack-a".to_string(),
                    trunk: "main".to_string(),
                    layers: Vec::new(),
                    is_current: false,
                },
                StackSummary {
                    label: "stack-b".to_string(),
                    trunk: "main".to_string(),
                    layers: Vec::new(),
                    is_current: false,
                },
            ],
            screen: Screen::List,
            status: None,
            layer_details: Default::default(),
            layer_diffs: Default::default(),
            should_quit: false,
        }
    }

    #[test]
    fn stacks_loaded_selects_given_index() {
        let mut component = StackList::new();
        let mut state = app_state();

        component.update(&Action::StacksLoaded(Some(1)), &mut state);

        assert_eq!(component.list_state.selected(), Some(1));
    }

    #[test]
    fn stacks_loaded_none_clears_selection() {
        let mut component = StackList::new();
        let mut state = app_state();
        component.update(&Action::StacksLoaded(Some(1)), &mut state);

        component.update(&Action::StacksLoaded(None), &mut state);

        assert_eq!(component.list_state.selected(), None);
    }
}
