use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::shell::{Shell, ShellError};

/// A pull request linked to a [`Layer`](super::Layer).
///
/// `number`, `url`, and `state` come straight from `gh stack view --json`'s
/// nested `pr` object. `title`, `is_draft`, `checks_status`, and
/// `review_decision` aren't present there — they're only available from
/// `gh pr view --json` (as `title`, `isDraft`, a derived checks summary, and
/// `reviewDecision`) and are filled in by a separate hydration step, so they
/// stay `None` until that runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestRef {
    pub number: u64,
    pub url: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_draft: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_decision: Option<String>,
}

/// Raw shape of `gh pr view --json number,url,state,title,isDraft,reviewDecision`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPrView {
    number: u64,
    url: String,
    state: String,
    title: String,
    is_draft: bool,
    review_decision: String,
}

impl From<GhPrView> for PullRequestRef {
    fn from(view: GhPrView) -> Self {
        PullRequestRef {
            number: view.number,
            url: view.url,
            state: view.state,
            title: Some(view.title),
            is_draft: Some(view.is_draft),
            checks_status: None,
            review_decision: Some(view.review_decision),
        }
    }
}

/// Looks up the pull request for `branch` via `gh pr view --json`.
///
/// Returns `Ok(None)` when the branch has no linked pull request — `gh pr
/// view` exits non-zero in that case, which isn't a real error for callers
/// enumerating stacks that may have unsubmitted layers.
pub async fn hydrate_pull_request(
    shell: &impl Shell,
    repo: &Path,
    branch: &str,
) -> Result<Option<PullRequestRef>, ShellError> {
    let result = shell
        .run(
            repo,
            "gh",
            &[
                "pr",
                "view",
                branch,
                "--json",
                "number,url,state,title,isDraft,reviewDecision",
            ],
        )
        .await;

    let output = match result {
        Ok(output) => output,
        Err(ShellError::CommandFailed { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };

    let view: GhPrView = serde_json::from_str(&output.stdout)
        .map_err(|error| ShellError::UnexpectedOutput(error.to_string()))?;

    Ok(Some(view.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{MockShell, ShellOutput};

    #[tokio::test]
    async fn hydrates_a_pull_request_from_gh_pr_view() {
        let repo = std::env::current_dir().unwrap();
        let shell = MockShell::new().when(
            "gh",
            &[
                "pr",
                "view",
                "layer-1",
                "--json",
                "number,url,state,title,isDraft,reviewDecision",
            ],
            Ok(ShellOutput {
                stdout: r#"{"number":44,"url":"https://github.com/o/r/pull/44","state":"OPEN","title":"stack demo/layer 1","isDraft":true,"reviewDecision":""}"#.to_string(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );

        let pr = hydrate_pull_request(&shell, repo.as_path(), "layer-1")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(pr.number, 44);
        assert_eq!(pr.title.as_deref(), Some("stack demo/layer 1"));
        assert_eq!(pr.is_draft, Some(true));
        assert_eq!(pr.review_decision.as_deref(), Some(""));
    }

    #[tokio::test]
    async fn returns_none_when_the_branch_has_no_pull_request() {
        let repo = std::env::current_dir().unwrap();
        let shell = MockShell::new().when(
            "gh",
            &[
                "pr",
                "view",
                "unsubmitted",
                "--json",
                "number,url,state,title,isDraft,reviewDecision",
            ],
            Err(ShellError::CommandFailed {
                program: "gh".to_string(),
                output: ShellOutput {
                    stdout: String::new(),
                    stderr: "no pull requests found".to_string(),
                    exit_code: 1,
                },
            }),
        );

        let pr = hydrate_pull_request(&shell, repo.as_path(), "unsubmitted")
            .await
            .unwrap();

        assert_eq!(pr, None);
    }

    #[tokio::test]
    async fn propagates_other_shell_errors() {
        let repo = std::env::current_dir().unwrap();
        let shell = MockShell::new().when(
            "gh",
            &[
                "pr",
                "view",
                "layer-1",
                "--json",
                "number,url,state,title,isDraft,reviewDecision",
            ],
            Err(ShellError::BinaryNotFound("gh".to_string())),
        );

        let result = hydrate_pull_request(&shell, repo.as_path(), "layer-1").await;

        assert!(matches!(result, Err(ShellError::BinaryNotFound(_))));
    }

    /// A `PullRequestRef` after hydration, in *our* wire shape (this
    /// struct's own field names — not raw `gh pr view --json`, which uses
    /// `isDraft`/`reviewDecision`). Values are the real ones captured for
    /// PR #44; `checks_status` is omitted since no hydration step computes
    /// it from `statusCheckRollup` yet.
    const HYDRATED_FIXTURE: &str = include_str!("fixtures/pr_ref_hydrated.json");

    #[test]
    fn parses_a_hydrated_pull_request_ref() {
        let pr: PullRequestRef = serde_json::from_str(HYDRATED_FIXTURE).unwrap();

        assert_eq!(pr.number, 44);
        assert_eq!(pr.url, "https://github.com/jordanleeevans/trellis/pull/44");
        assert_eq!(pr.state, "OPEN");
        assert_eq!(pr.title.as_deref(), Some("stack demo/layer 1"));
        assert_eq!(pr.is_draft, Some(true));
        assert_eq!(pr.checks_status, None);
        assert_eq!(pr.review_decision.as_deref(), Some(""));
    }

    #[test]
    fn round_trips_a_hydrated_pull_request_ref_without_loss() {
        let pr: PullRequestRef = serde_json::from_str(HYDRATED_FIXTURE).unwrap();

        let reserialized: serde_json::Value = serde_json::to_value(&pr).unwrap();
        let original: serde_json::Value = serde_json::from_str(HYDRATED_FIXTURE).unwrap();

        assert_eq!(reserialized, original);
    }

    #[test]
    fn parses_the_thin_ref_from_stack_view_with_no_optional_fields() {
        let json = r#"{"number":44,"url":"https://github.com/jordanleeevans/trellis/pull/44","state":"OPEN"}"#;

        let pr: PullRequestRef = serde_json::from_str(json).unwrap();

        assert_eq!(pr.number, 44);
        assert_eq!(pr.title, None);
        assert_eq!(pr.is_draft, None);
        assert_eq!(pr.checks_status, None);
        assert_eq!(pr.review_decision, None);
    }
}
