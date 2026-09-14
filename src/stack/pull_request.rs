use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
