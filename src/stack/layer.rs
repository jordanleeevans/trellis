use serde::{Deserialize, Serialize};

use super::commit::CommitInfo;
use super::pull_request::PullRequestRef;

/// One branch in a [`Stack`](super::Stack), as reported by
/// `gh stack view --json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    #[serde(rename = "name")]
    pub branch: String,
    /// The branch's tip commit. Absent until the branch has been pushed
    /// (e.g. via `gh stack submit`) — a freshly `gh stack init`-ed layer has
    /// no `head` in the JSON at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    pub base: String,
    pub is_current: bool,
    pub is_merged: bool,
    pub is_queued: bool,
    pub needs_rebase: bool,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "pr")]
    pub pull_request: Option<PullRequestRef>,
    /// Hydrated separately from `gh pr view --json commits` — never present
    /// in `gh stack view --json`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commits: Vec<CommitInfo>,
    /// Index within the stack, closest-to-trunk first. Computed from array
    /// order after parsing — never part of the wire JSON.
    #[serde(skip)]
    pub position: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One entry from the real, submitted `gh stack view --json` fixture.
    const SUBMITTED_LAYER: &str = r#"{
        "name": "stack-demo/layer-1",
        "head": "60b4fd24f37bd8df27a8385ec665ca4faa56b00b",
        "base": "18aeb0a5b395fd932f9c5396d8e34039ef6f99f6",
        "isCurrent": false,
        "isMerged": false,
        "isQueued": false,
        "needsRebase": true,
        "pr": {
            "number": 44,
            "url": "https://github.com/jordanleeevans/trellis/pull/44",
            "state": "OPEN"
        }
    }"#;

    /// One entry from the real, unsubmitted `gh stack view --json` fixture
    /// — no `head`, no `pr`.
    const UNSUBMITTED_LAYER: &str = r#"{
        "name": "stack-demo/layer-1",
        "base": "18aeb0a5b395fd932f9c5396d8e34039ef6f99f6",
        "isCurrent": false,
        "isMerged": false,
        "isQueued": false,
        "needsRebase": true
    }"#;

    #[test]
    fn parses_a_submitted_layer() {
        let layer: Layer = serde_json::from_str(SUBMITTED_LAYER).unwrap();

        assert_eq!(layer.branch, "stack-demo/layer-1");
        assert_eq!(
            layer.head.as_deref(),
            Some("60b4fd24f37bd8df27a8385ec665ca4faa56b00b")
        );
        assert!(layer.needs_rebase);
        assert_eq!(layer.pull_request.as_ref().unwrap().number, 44);
        assert!(layer.commits.is_empty());
        assert_eq!(layer.position, 0);
    }

    #[test]
    fn round_trips_a_submitted_layer_without_loss() {
        let layer: Layer = serde_json::from_str(SUBMITTED_LAYER).unwrap();

        let reserialized: serde_json::Value = serde_json::to_value(&layer).unwrap();
        let original: serde_json::Value = serde_json::from_str(SUBMITTED_LAYER).unwrap();

        assert_eq!(reserialized, original);
    }

    #[test]
    fn parses_an_unsubmitted_layer() {
        let layer: Layer = serde_json::from_str(UNSUBMITTED_LAYER).unwrap();

        assert_eq!(layer.head, None);
        assert_eq!(layer.pull_request, None);
    }

    #[test]
    fn round_trips_an_unsubmitted_layer_without_loss() {
        let layer: Layer = serde_json::from_str(UNSUBMITTED_LAYER).unwrap();

        let reserialized: serde_json::Value = serde_json::to_value(&layer).unwrap();
        let original: serde_json::Value = serde_json::from_str(UNSUBMITTED_LAYER).unwrap();

        assert_eq!(reserialized, original);
    }
}
