use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use ratatui::Frame;

use crate::shell::Shell;
use crate::stack::{StackSummary, list_stacks};

use super::stack_layers;
use super::stack_list;

/// Which screen is currently shown.
#[derive(Debug, Clone, Copy)]
pub enum Screen {
    /// The entry-point panel: every locally tracked stack.
    List,
    /// The layer view for the stack at this index into [`AppState::stacks`].
    Layers(usize),
}

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    RefreshStacks,
    SelectNext,
    SelectPrevious,
    ShowLayers(usize),
    ShowList,
    OpenPullRequest {
        stack_index: usize,
        layer_index: usize,
    },
    StacksLoaded(Option<usize>),
    SetStatus(String),
    ClearStatus,
}

pub trait Component {
    fn draw(&mut self, frame: &mut Frame, state: &AppState);
    fn handle_key(&mut self, code: KeyCode, state: &AppState) -> Vec<Action>;
    fn update(&mut self, action: &Action, state: &mut AppState);
}

pub struct AppState {
    pub stacks: Vec<StackSummary>,
    pub screen: Screen,
    pub status: Option<String>,
    pub(crate) should_quit: bool,
}

struct App {
    state: AppState,
    stack_list: stack_list::StackList,
    stack_layers: stack_layers::StackLayers,
}

impl AppState {
    fn new() -> Self {
        Self {
            stacks: Vec::new(),
            screen: Screen::List,
            status: None,
            should_quit: false,
        }
    }
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState::new(),
            stack_list: stack_list::StackList::new(),
            stack_layers: stack_layers::StackLayers::new(),
        }
    }

    /// Reloads every locally tracked stack, preserving the current
    /// selection (by stack) where possible.
    async fn refresh(&mut self, shell: &impl Shell, repo: &Path) -> Vec<Action> {
        let selected_label = match self.state.screen {
            Screen::Layers(index) => self
                .state
                .stacks
                .get(index)
                .map(|stack| stack.label.clone()),
            Screen::List => self
                .stack_list
                .selected_index()
                .and_then(|index| self.state.stacks.get(index))
                .map(|stack| stack.label.clone()),
        };

        match list_stacks(shell, repo).await {
            Ok(stacks) => {
                let selected_index = selected_label
                    .and_then(|label| stacks.iter().position(|stack| stack.label == label))
                    .or_else(|| stacks.iter().position(|stack| stack.is_current))
                    .or(if stacks.is_empty() { None } else { Some(0) });

                self.state.stacks = stacks;
                let mut actions = vec![Action::StacksLoaded(selected_index)];

                if matches!(self.state.screen, Screen::Layers(_)) {
                    if let Some(index) = selected_index {
                        self.state.screen = Screen::Layers(index);
                        actions.push(Action::ClearStatus);
                    } else {
                        self.state.screen = Screen::List;
                        actions.push(Action::SetStatus(
                            "selected stack is no longer available".to_string(),
                        ));
                    }
                } else {
                    actions.push(Action::ClearStatus);
                }
                actions
            }
            Err(error) => vec![Action::SetStatus(format!("failed to load stacks: {error}"))],
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        match self.state.screen {
            Screen::List => self.stack_list.draw(frame, &self.state),
            Screen::Layers(_) => self.stack_layers.draw(frame, &self.state),
        }
    }

    fn handle_key(&mut self, code: KeyCode) -> Vec<Action> {
        match self.state.screen {
            Screen::List => self.stack_list.handle_key(code, &self.state),
            Screen::Layers(_) => self.stack_layers.handle_key(code, &self.state),
        }
    }

    async fn dispatch_actions(&mut self, actions: Vec<Action>, shell: &impl Shell, repo: &Path) {
        let mut pending = std::collections::VecDeque::from(actions);

        while let Some(action) = pending.pop_front() {
            let follow_ups = self.apply_action(&action, shell, repo).await;
            self.stack_list.update(&action, &mut self.state);
            self.stack_layers.update(&action, &mut self.state);
            pending.extend(follow_ups);
        }
    }

    async fn apply_action(
        &mut self,
        action: &Action,
        shell: &impl Shell,
        repo: &Path,
    ) -> Vec<Action> {
        match action {
            Action::Quit => {
                self.state.should_quit = true;
                Vec::new()
            }
            Action::RefreshStacks => self.refresh(shell, repo).await,
            Action::ShowLayers(index) => {
                if *index < self.state.stacks.len() {
                    self.state.screen = Screen::Layers(*index);
                    if self
                        .state
                        .status
                        .as_deref()
                        .is_some_and(|status| status.starts_with("failed to load stacks:"))
                    {
                        self.state.status = None;
                    }
                } else {
                    self.state.screen = Screen::List;
                    self.state.status = Some("selected stack is no longer available".to_string());
                }
                Vec::new()
            }
            Action::ShowList => {
                self.state.screen = Screen::List;
                Vec::new()
            }
            Action::OpenPullRequest {
                stack_index,
                layer_index,
            } => {
                if let Some(pr) = self
                    .state
                    .stacks
                    .get(*stack_index)
                    .and_then(|stack| stack.layers.get(*layer_index))
                    .and_then(|layer| layer.pull_request.as_ref())
                {
                    let pr_number = pr.number.to_string();
                    if let Err(error) = shell
                        .run(repo, "gh", &["pr", "view", &pr_number, "--web"])
                        .await
                    {
                        self.state.status = Some(format!("failed to open PR: {error}"));
                    }
                } else {
                    self.state.status = Some("selected layer has no pull request".to_string());
                }
                Vec::new()
            }
            Action::SetStatus(message) => {
                self.state.status = Some(message.clone());
                Vec::new()
            }
            Action::ClearStatus => {
                self.state.status = None;
                Vec::new()
            }
            Action::StacksLoaded(_) => {
                if let Screen::Layers(index) = self.state.screen
                    && index >= self.state.stacks.len()
                {
                    self.state.screen = Screen::List;
                    self.state.status = Some("selected stack is no longer available".to_string());
                }
                Vec::new()
            }
            Action::SelectNext | Action::SelectPrevious => Vec::new(),
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
    let initial_actions = app.refresh(shell, repo).await;
    app.dispatch_actions(initial_actions, shell, repo).await;

    while !app.state.should_quit {
        terminal.draw(|frame| app.draw(frame))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            let actions = app.handle_key(key.code);
            app.dispatch_actions(actions, shell, repo).await;
        }
    }

    Ok(())
}
