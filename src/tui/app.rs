use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use ratatui::widgets::ListState;

use crate::shell::Shell;
use crate::stack::{StackSummary, list_stacks};

use super::stack_layers;
use super::stack_list;

/// Which screen is currently shown.
#[derive(Debug, Clone, Copy)]
pub enum Screen {
    /// The entry-point panel: every locally tracked stack.
    List,
    /// The layer view for the stack at this index into [`App::stacks`].
    Layers(usize),
}

pub struct App {
    pub stacks: Vec<StackSummary>,
    pub stack_list_state: ListState,
    pub layer_list_state: ListState,
    pub screen: Screen,
    pub status: Option<String>,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            stacks: Vec::new(),
            stack_list_state: ListState::default(),
            layer_list_state: ListState::default(),
            screen: Screen::List,
            status: None,
            should_quit: false,
        }
    }

    /// Reloads every locally tracked stack, preserving the current
    /// selection (by stack) where possible.
    async fn refresh(&mut self, shell: &impl Shell, repo: &Path) {
        let selected_label = self
            .stack_list_state
            .selected()
            .and_then(|index| self.stacks.get(index))
            .map(|stack| stack.label.clone());

        match list_stacks(shell, repo).await {
            Ok(stacks) => {
                let selected_index = selected_label
                    .and_then(|label| stacks.iter().position(|stack| stack.label == label))
                    .or_else(|| stacks.iter().position(|stack| stack.is_current))
                    .or(if stacks.is_empty() { None } else { Some(0) });

                self.stacks = stacks;
                self.stack_list_state.select(selected_index);
                self.status = None;
            }
            Err(error) => {
                self.status = Some(format!("failed to load stacks: {error}"));
            }
        }
    }
}

/// Runs the TUI until the user quits, then restores the terminal.
pub async fn run(shell: &impl Shell, repo: &Path) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, shell, repo).await;
    ratatui::restore();
    result
}

/// Runs the app
async fn run_app(
    terminal: &mut DefaultTerminal,
    shell: &impl Shell,
    repo: &Path,
) -> anyhow::Result<()> {
    let mut app = App::new();
    app.refresh(shell, repo).await;

    while !app.should_quit {
        terminal.draw(|frame| match app.screen {
            Screen::List => stack_list::render(frame, &app),
            Screen::Layers(index) => stack_layers::render(frame, &mut app, index),
        })?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            handle_key(&mut app, key.code, shell, repo).await;
        }
    }

    Ok(())
}

/// Handles key presses based on the currently rendered frame
async fn handle_key(app: &mut App, code: KeyCode, shell: &impl Shell, repo: &Path) {
    match app.screen {
        Screen::List => match code {
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            KeyCode::Char('r') => app.refresh(shell, repo).await,
            KeyCode::Down | KeyCode::Char('j') => {
                select_next(&mut app.stack_list_state, app.stacks.len())
            }
            KeyCode::Up | KeyCode::Char('k') => {
                select_previous(&mut app.stack_list_state, app.stacks.len())
            }
            KeyCode::Enter => {
                if let Some(index) = app.stack_list_state.selected() {
                    app.screen = Screen::Layers(index);
                    app.layer_list_state.select(Some(0))
                }
            }
            _ => {}
        },
        Screen::Layers(stack_index) => {
            let layer_count = app
                .stacks
                .get(stack_index)
                .map(|stack| stack.layers.len())
                .unwrap_or(0);

            match code {
                KeyCode::Char('q') | KeyCode::Esc => app.screen = Screen::List,
                KeyCode::Down | KeyCode::Char('j') => {
                    select_next(&mut app.layer_list_state, layer_count)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    select_previous(&mut app.layer_list_state, layer_count)
                }
                KeyCode::Char('O') => {
                    if let Some(layer_index) = app.layer_list_state.selected()
                        && let Some(layer) = app
                            .stacks
                            .get(stack_index)
                            .and_then(|stack| stack.layers.get(layer_index))
                        && let Some(pr) = &layer.pull_request
                    {
                        let pr_number = pr.number.to_string();

                        let _ = shell
                            .run(repo, "gh", &["pr", "view", &pr_number, "--web"])
                            .await;
                    }
                }
                _ => {}
            }
        }
    };
}

/// Selects the next element in [ListState]
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

/// Selects the previous element in [ListState]
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
