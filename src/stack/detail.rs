use std::path::Path;

use serde::Deserialize;

use crate::shell::{Shell, ShellError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerDetail {
    pub commits: Vec<LayerCommit>,
    pub pull_request: PullRequestDetail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerCommit {
    pub oid: String,
    pub subject: String,
    pub author: Option<String>,
    pub authored_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestDetail {
    pub title: String,
    pub description_snippet: Option<String>,
    pub reviewers: Vec<ReviewerState>,
    pub checks: CheckSummary,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerState {
    pub login: String,
    pub state: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheckSummary {
    pub total: usize,
    pub passing: usize,
    pub failing: usize,
    pub pending: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPrDetail {
    title: String,
    body: String,
    commits: Vec<GhCommit>,
    reviews: Vec<GhReview>,
    review_requests: Vec<GhReviewRequest>,
    status_check_rollup: Vec<GhStatusCheck>,
    labels: Vec<GhLabel>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhCommit {
    oid: String,
    message_headline: String,
    authored_date: String,
    #[serde(default)]
    authors: Vec<GhAuthor>,
}

#[derive(Debug, Deserialize)]
struct GhAuthor {
    login: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhReview {
    author: GhReviewAuthor,
    state: String,
}

#[derive(Debug, Deserialize)]
struct GhReviewAuthor {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GhReviewRequest {
    #[serde(rename = "requestedReviewer")]
    requested_reviewer: GhRequestedReviewer,
}

#[derive(Debug, Deserialize)]
struct GhRequestedReviewer {
    login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhStatusCheck {
    state: Option<String>,
    conclusion: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhLabel {
    name: String,
}

impl LayerDetail {
    pub fn from_gh_pr_view(raw: &str) -> serde_json::Result<Self> {
        let detail: GhPrDetail = serde_json::from_str(raw)?;
        Ok(detail.into())
    }
}

impl From<GhPrDetail> for LayerDetail {
    fn from(detail: GhPrDetail) -> Self {
        LayerDetail {
            commits: detail.commits.into_iter().map(Into::into).collect(),
            pull_request: PullRequestDetail {
                title: detail.title,
                description_snippet: description_snippet(&detail.body),
                reviewers: reviewers(detail.reviews, detail.review_requests),
                checks: CheckSummary::from_checks(&detail.status_check_rollup),
                labels: detail.labels.into_iter().map(|label| label.name).collect(),
            },
        }
    }
}

impl From<GhCommit> for LayerCommit {
    fn from(commit: GhCommit) -> Self {
        let author = commit
            .authors
            .first()
            .and_then(|author| author.login.clone().or_else(|| author.name.clone()));

        LayerCommit {
            oid: commit.oid,
            subject: commit.message_headline,
            author,
            authored_at: commit.authored_date,
        }
    }
}

impl CheckSummary {
    fn from_checks(checks: &[GhStatusCheck]) -> Self {
        let mut summary = CheckSummary {
            total: checks.len(),
            ..CheckSummary::default()
        };

        for check in checks {
            match (
                check.state.as_deref().unwrap_or_default(),
                check.conclusion.as_deref().unwrap_or_default(),
            ) {
                ("SUCCESS", _) | ("COMPLETED", "SUCCESS") => summary.passing += 1,
                ("FAILURE", _) | ("ERROR", _) | ("COMPLETED", "FAILURE" | "ERROR") => {
                    summary.failing += 1
                }
                _ => summary.pending += 1,
            }
        }

        summary
    }
}

fn description_snippet(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.chars().take(180).collect())
    }
}

fn reviewers(reviews: Vec<GhReview>, requests: Vec<GhReviewRequest>) -> Vec<ReviewerState> {
    let mut reviewers: Vec<ReviewerState> = reviews
        .into_iter()
        .map(|review| ReviewerState {
            login: review.author.login,
            state: review.state,
        })
        .collect();

    reviewers.extend(requests.into_iter().map(|request| ReviewerState {
        login: request.requested_reviewer.login,
        state: "REQUESTED".to_string(),
    }));

    reviewers
}

pub async fn hydrate_layer_detail(
    shell: &impl Shell,
    repo: &Path,
    branch: &str,
) -> Result<LayerDetail, ShellError> {
    let output = shell
        .run(
            repo,
            "gh",
            &[
                "pr",
                "view",
                branch,
                "--json",
                "title,body,commits,reviews,reviewRequests,statusCheckRollup,labels",
            ],
        )
        .await?;

    let detail = LayerDetail::from_gh_pr_view(&output.stdout)
        .map_err(|error| ShellError::UnexpectedOutput(error.to_string()))?;

    Ok(detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PR_DETAIL: &str = r#"{
        "title": "Add layer detail pane",
        "body": "Shows commits and PR metadata for the selected layer.\n\nLonger body follows.",
        "commits": [
            {
                "oid": "60b4fd24f37bd8df27a8385ec665ca4faa56b00b",
                "messageHeadline": "feat: layer detail",
                "authoredDate": "2026-09-14T23:03:34Z",
                "authors": [
                    { "login": "john-doe", "name": "John Doe" }
                ]
            }
        ],
        "reviews": [
            {
                "author": { "login": "octocat" },
                "state": "APPROVED"
            }
        ],
        "reviewRequests": [
            {
                "requestedReviewer": { "login": "mona" }
            }
        ],
        "statusCheckRollup": [
            { "state": "SUCCESS", "conclusion": null },
            { "state": "COMPLETED", "conclusion": "FAILURE" },
            { "state": "QUEUED", "conclusion": null }
        ],
        "labels": [
            { "name": "enhancement" },
            { "name": "tui" }
        ]
    }"#;

    #[test]
    fn parses_layer_detail_from_gh_pr_view_json() {
        let detail = LayerDetail::from_gh_pr_view(PR_DETAIL).unwrap();

        assert_eq!(detail.pull_request.title, "Add layer detail pane");
        assert_eq!(
            detail.pull_request.description_snippet.as_deref(),
            Some("Shows commits and PR metadata for the selected layer.\n\nLonger body follows.")
        );
        assert_eq!(
            detail.commits,
            vec![LayerCommit {
                oid: "60b4fd24f37bd8df27a8385ec665ca4faa56b00b".to_string(),
                subject: "feat: layer detail".to_string(),
                author: Some("john-doe".to_string()),
                authored_at: "2026-09-14T23:03:34Z".to_string(),
            }]
        );
        assert_eq!(
            detail.pull_request.reviewers,
            vec![
                ReviewerState {
                    login: "octocat".to_string(),
                    state: "APPROVED".to_string(),
                },
                ReviewerState {
                    login: "mona".to_string(),
                    state: "REQUESTED".to_string(),
                },
            ]
        );
        assert_eq!(
            detail.pull_request.checks,
            CheckSummary {
                total: 3,
                passing: 1,
                failing: 1,
                pending: 1,
            }
        );
        assert_eq!(detail.pull_request.labels, vec!["enhancement", "tui"]);
    }

    #[test]
    fn omits_description_snippet_for_blank_body() {
        assert_eq!(description_snippet(" \n\t "), None);
    }
}
