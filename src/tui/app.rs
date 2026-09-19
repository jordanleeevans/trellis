use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use ratatui::Frame;
use tokio::sync::mpsc;

use crate::git;
use crate::shell::{ProcessShell, Shell};
use crate::stack::{Layer, LayerDetail, StackSummary, hydrate_layer_detail, list_stacks};

use super::layer_resource::LayerResourceCache;
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
    FocusNextPanel,
    FocusPreviousPanel,
    SelectNextDiffFile,
    SelectPreviousDiffFile,
    ScrollDiffLineDown,
    ScrollDiffLineUp,
    ScrollDiffDown,
    ScrollDiffUp,
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
    LoadLayerDiff {
        stack_index: usize,
        layer_index: usize,
        force: bool,
    },
    LayerDetailLoaded {
        cache_key: String,
        result: Result<LayerDetail, String>,
    },
    LayerDiffLoaded {
        cache_key: String,
        result: Result<String, String>,
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
    pub layer_details: LayerResourceCache<LayerDetail>,
    pub layer_diffs: LayerResourceCache<String>,
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
            layer_details: LayerResourceCache::default(),
            layer_diffs: LayerResourceCache::default(),
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

    async fn dispatch_actions_with_loader(
        &mut self,
        actions: Vec<Action>,
        shell: &impl Shell,
        repo: &Path,
        loader: Option<&LayerLoadScheduler>,
    ) {
        let mut pending = std::collections::VecDeque::from(actions);

        while let Some(action) = pending.pop_front() {
            let follow_ups = if let Some(loader) = loader {
                if self.schedule_layer_load(&action, loader) {
                    Vec::new()
                } else {
                    self.apply_action(&action, shell, repo).await
                }
            } else {
                self.apply_action(&action, shell, repo).await
            };
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
                pending.push_back(Action::LoadLayerDiff {
                    stack_index,
                    layer_index,
                    force: false,
                });
            }
        }
    }

    fn schedule_layer_load(&mut self, action: &Action, loader: &LayerLoadScheduler) -> bool {
        match action {
            Action::LoadLayerDetail {
                stack_index,
                layer_index,
                force,
            } => {
                let Some((cache_key, branch, has_pull_request)) =
                    self.layer_load_context(*stack_index, *layer_index)
                else {
                    self.state.status = Some("selected layer is no longer available".to_string());
                    return true;
                };

                if !has_pull_request {
                    return true;
                }

                if !self.state.layer_details.should_load(&cache_key, *force) {
                    return true;
                }

                self.state.layer_details.mark_loading(cache_key.clone());
                loader.load_detail(cache_key, branch);
                true
            }
            Action::LoadLayerDiff {
                stack_index,
                layer_index,
                force,
            } => {
                let Some((cache_key, branch, lower)) =
                    self.layer_diff_context(*stack_index, *layer_index)
                else {
                    self.state.status = Some("selected layer is no longer available".to_string());
                    return true;
                };

                if !self.state.layer_diffs.should_load(&cache_key, *force) {
                    return true;
                }

                self.state.layer_diffs.mark_loading(cache_key.clone());
                loader.load_diff(cache_key, lower, branch);
                true
            }
            _ => false,
        }
    }

    fn layer_load_context(
        &self,
        stack_index: usize,
        layer_index: usize,
    ) -> Option<(String, String, bool)> {
        let stack = self.state.stacks.get(stack_index)?;
        let layer = stack.layers.get(layer_index)?;
        Some((
            layer_detail_cache_key(stack, layer),
            layer.branch.clone(),
            layer.pull_request.is_some(),
        ))
    }

    fn layer_diff_context(
        &self,
        stack_index: usize,
        layer_index: usize,
    ) -> Option<(String, String, String)> {
        let stack = self.state.stacks.get(stack_index)?;
        let layer = stack.layers.get(layer_index)?;
        Some((
            layer_diff_cache_key(stack, layer),
            layer.branch.clone(),
            lower_layer_ref(stack, layer_index),
        ))
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

                    vec![
                        Action::LoadLayerDetail {
                            stack_index: *index,
                            layer_index: 0,
                            force: false,
                        },
                        Action::LoadLayerDiff {
                            stack_index: *index,
                            layer_index: 0,
                            force: false,
                        },
                    ]
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
                if !self.state.layer_details.should_load(&cache_key, *force) {
                    return Vec::new();
                }

                match hydrate_layer_detail(shell, repo, &layer.branch).await {
                    Ok(detail) => {
                        self.state.layer_details.store_result(cache_key, Ok(detail));
                    }
                    Err(error) => {
                        let message = error.to_string();
                        self.state
                            .layer_details
                            .store_result(cache_key, Err(message.clone()));
                        self.state.status = Some(format!("failed to load layer detail: {message}"));
                    }
                }

                Vec::new()
            }
            Action::LayerDetailLoaded { cache_key, result } => {
                if let Err(error) = result {
                    self.state.status = Some(format!("failed to load layer detail: {error}"));
                }
                self.state
                    .layer_details
                    .store_result(cache_key.clone(), result.clone());
                Vec::new()
            }
            Action::LoadLayerDiff {
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

                let cache_key = layer_diff_cache_key(stack, layer);
                if !self.state.layer_diffs.should_load(&cache_key, *force) {
                    return Vec::new();
                }

                let lower = lower_layer_ref(stack, *layer_index);
                match git::diff(shell, repo, &lower, &layer.branch).await {
                    Ok(diff) => {
                        self.state.layer_diffs.store_result(cache_key, Ok(diff));
                    }
                    Err(error) => {
                        let message = error.to_string();
                        self.state
                            .layer_diffs
                            .store_result(cache_key, Err(message.clone()));
                        self.state.status = Some(format!("failed to load layer diff: {message}"));
                    }
                }

                Vec::new()
            }
            Action::LayerDiffLoaded { cache_key, result } => {
                if let Err(error) = result {
                    self.state.status = Some(format!("failed to load layer diff: {error}"));
                }
                self.state
                    .layer_diffs
                    .store_result(cache_key.clone(), result.clone());
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
            Action::SelectNext
            | Action::SelectPrevious
            | Action::FocusNextPanel
            | Action::FocusPreviousPanel
            | Action::SelectNextDiffFile
            | Action::SelectPreviousDiffFile
            | Action::ScrollDiffLineDown
            | Action::ScrollDiffLineUp
            | Action::ScrollDiffDown
            | Action::ScrollDiffUp => Vec::new(),
        }
    }
}

struct LayerLoadScheduler {
    repo: PathBuf,
    tx: mpsc::UnboundedSender<Action>,
}

impl LayerLoadScheduler {
    fn new(repo: PathBuf, tx: mpsc::UnboundedSender<Action>) -> Self {
        Self { repo, tx }
    }

    fn load_detail(&self, cache_key: String, branch: String) {
        let repo = self.repo.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = hydrate_layer_detail(&ProcessShell, repo.as_path(), &branch)
                .await
                .map_err(|error| error.to_string());
            let _ = tx.send(Action::LayerDetailLoaded { cache_key, result });
        });
    }

    fn load_diff(&self, cache_key: String, lower: String, branch: String) {
        let repo = self.repo.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = git::diff(&ProcessShell, repo.as_path(), &lower, &branch)
                .await
                .map_err(|error| error.to_string());
            let _ = tx.send(Action::LayerDiffLoaded { cache_key, result });
        });
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
    let (tx, mut rx) = mpsc::unbounded_channel();
    let loader = LayerLoadScheduler::new(repo.to_path_buf(), tx);
    let initial_actions = app.refresh(shell, repo).await;
    app.dispatch_actions_with_loader(initial_actions, shell, repo, Some(&loader))
        .await;

    while !app.state.should_quit {
        while let Ok(action) = rx.try_recv() {
            app.dispatch_actions_with_loader(vec![action], shell, repo, Some(&loader))
                .await;
        }

        terminal.draw(|frame| app.draw(frame))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            let actions = app.handle_key(key.code);
            app.dispatch_actions_with_loader(actions, shell, repo, Some(&loader))
                .await;
        }
    }

    Ok(())
}

pub fn layer_detail_cache_key(stack: &StackSummary, layer: &Layer) -> String {
    format!("{}::{}", stack.label, layer.branch)
}

pub fn layer_diff_cache_key(stack: &StackSummary, layer: &Layer) -> String {
    format!("{}::{}::diff", stack.label, layer.branch)
}

pub fn lower_layer_ref(stack: &StackSummary, layer_index: usize) -> String {
    stack
        .layers
        .get(layer_index.saturating_sub(1))
        .filter(|_| layer_index > 0)
        .map(|layer| layer.branch.clone())
        .unwrap_or_else(|| stack.trunk.clone())
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

    #[test]
    fn layer_diff_cache_key_includes_stack_and_branch() {
        let stack = stack_summary("stack-a", 1);
        let layer = layer("feature/layer-1");

        assert_eq!(
            layer_diff_cache_key(&stack, &layer),
            "stack-a::feature/layer-1::diff"
        );
    }

    #[test]
    fn lower_layer_ref_uses_trunk_for_bottom_layer() {
        let stack = stack_summary("stack-a", 2);

        assert_eq!(lower_layer_ref(&stack, 0), "main");
    }

    #[test]
    fn lower_layer_ref_uses_previous_layer_for_higher_layers() {
        let stack = stack_summary("stack-a", 2);

        assert_eq!(lower_layer_ref(&stack, 1), "stack-a-layer-0");
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
            [
                Action::LoadLayerDetail {
                    stack_index: 0,
                    layer_index: 0,
                    force: false,
                },
                Action::LoadLayerDiff {
                    stack_index: 0,
                    layer_index: 0,
                    force: false,
                }
            ]
        ));
    }

    #[tokio::test]
    async fn load_layer_diff_diffs_bottom_layer_against_trunk() {
        let repo = std::env::current_dir().unwrap();
        let shell = MockShell::new().when(
            "git",
            &[
                "diff",
                "--color=never",
                "--find-renames",
                "main..stack-a-layer-0",
            ],
            Ok(crate::shell::ShellOutput {
                stdout: "+bottom".to_string(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let mut app = App::new();
        app.state.stacks = vec![stack_summary("stack-a", 2)];

        app.apply_action(
            &Action::LoadLayerDiff {
                stack_index: 0,
                layer_index: 0,
                force: false,
            },
            &shell,
            repo.as_path(),
        )
        .await;

        let key = layer_diff_cache_key(&app.state.stacks[0], &app.state.stacks[0].layers[0]);
        assert_eq!(app.state.layer_diffs.get(&key).unwrap(), "+bottom");
    }
}
