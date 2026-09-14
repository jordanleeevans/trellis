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
pub enum Screen {
    /// The entry-point panel: every locally tracked stack.
    List,
    /// The layer view for the stack at this index into [`App::stacks`].
    Layers(usize),
}

pub struct App {
    pub stacks: Vec<StackSummary>,
    pub list_state: ListState,
    pub screen: Screen,
    pub status: Option<String>,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            stacks: Vec::new(),
            list_state: ListState::default(),
            screen: Screen::List,
            status: None,
            should_quit: false,
        }
    }

    /// Reloads every locally tracked stack, preserving the current
    /// selection (by stack) where possible.
    async fn refresh(&mut self, shell: &impl Shell, repo: &Path) {
        let selected_label = self
            .list_state
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
                self.list_state.select(selected_index);
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
            Screen::Layers(index) => stack_layers::render(frame, &app, index),
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

async fn handle_key(app: &mut App, code: KeyCode, shell: &impl Shell, repo: &Path) {
    match app.screen {
        Screen::List => match code {
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            KeyCode::Char('r') => app.refresh(shell, repo).await,
            KeyCode::Down | KeyCode::Char('j') => select_next(app),
            KeyCode::Up | KeyCode::Char('k') => select_previous(app),
            KeyCode::Enter => {
                if let Some(index) = app.list_state.selected() {
                    app.screen = Screen::Layers(index);
                }
            }
            _ => {}
        },
        Screen::Layers(_) => match code {
            KeyCode::Char('q') | KeyCode::Esc => app.screen = Screen::List,
            _ => {}
        },
    }
}

fn select_next(app: &mut App) {
    if app.stacks.is_empty() {
        return;
    }
    let next = match app.list_state.selected() {
        Some(index) if index + 1 < app.stacks.len() => index + 1,
        Some(_) => 0,
        None => 0,
    };
    app.list_state.select(Some(next));
}

fn select_previous(app: &mut App) {
    if app.stacks.is_empty() {
        return;
    }
    let previous = match app.list_state.selected() {
        Some(0) | None => app.stacks.len() - 1,
        Some(index) => index - 1,
    };
    app.list_state.select(Some(previous));
}
