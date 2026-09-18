use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use ratatui::Frame;

use crate::shell::Shell;
use crate::stack::{Layer, LayerDetail, StackSummary, hydrate_layer_detail, list_stacks};

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
    LoadLayerDetail {
        stack_index: usize,
        layer_index: usize,
        force: bool,
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
    pub layer_detail_cache: HashMap<String, LayerDetail>,
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
            layer_detail_cache: HashMap::new(),
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

            if matches!(action, Action::SelectNext | Action::SelectPrevious)
                && let Screen::Layers(stack_index) = self.state.screen
                && let Some(layer_index) = self.stack_layers.selected_index()
            {
                pending.push_back(Action::LoadLayerDetail {
                    stack_index,
                    layer_index,
                    force: false,
                });
            }
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

                    vec![Action::LoadLayerDetail {
                        stack_index: *index,
                        layer_index: 0,
                        force: false,
                    }]
                } else {
                    self.state.screen = Screen::List;
                    self.state.status = Some("selected stack is no longer available".to_string());
                    Vec::new()
                }
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
            Action::LoadLayerDetail {
                stack_index,
                layer_index,
                force,
            } => {
                let Some(stack) = self.state.stacks.get(*stack_index) else {
                    self.state.status = Some("selected stack is no longer available".to_string());
                    return Vec::new();
                };

                let Some(layer) = stack.layers.get(*layer_index) else {
                    self.state.status = Some("selected layer is no longer available".to_string());
                    return Vec::new();
                };

                if layer.pull_request.is_none() {
                    return Vec::new();
                }

                let cache_key = layer_detail_cache_key(stack, layer);
                if !force && self.state.layer_detail_cache.contains_key(&cache_key) {
                    return Vec::new();
                }

                match hydrate_layer_detail(shell, repo, &layer.branch).await {
                    Ok(detail) => {
                        self.state.layer_detail_cache.insert(cache_key, detail);
                    }
                    Err(error) => {
                        self.state.status = Some(format!("failed to load layer detail: {error}"));
                    }
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

pub fn layer_detail_cache_key(stack: &StackSummary, layer: &Layer) -> String {
    format!("{}::{}", stack.label, layer.branch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::MockShell;
    use crate::test_fixtures::{layer, stack_summary};

    #[test]
    fn layer_detail_cache_key_includes_stack_and_branch() {
        let stack = stack_summary("stack-a", 1);
        let layer = layer("feature/layer-1");

        assert_eq!(
            layer_detail_cache_key(&stack, &layer),
            "stack-a::feature/layer-1"
        );
    }

    #[tokio::test]
    async fn show_layers_loads_first_layer_detail() {
        let repo = std::env::current_dir().unwrap();
        let shell = MockShell::new();
        let mut app = App::new();
        app.state.stacks = vec![stack_summary("stack-a", 2)];

        let follow_ups = app
            .apply_action(&Action::ShowLayers(0), &shell, repo.as_path())
            .await;

        assert!(matches!(app.state.screen, Screen::Layers(0)));
        assert!(matches!(
            follow_ups.as_slice(),
            [Action::LoadLayerDetail {
                stack_index: 0,
                layer_index: 0,
                force: false,
            }]
        ));
    }
}
