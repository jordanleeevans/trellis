use serde::{Deserialize, Serialize};

/// A single commit belonging to a [`Layer`](super::Layer).
///
/// `gh stack view --json` doesn't include per-branch commits; this is
/// hydrated separately from `gh pr view --json commits` once a layer has a
/// linked pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitInfo {
    pub oid: String,
    #[serde(rename = "messageHeadline")]
    pub message: String,
    #[serde(rename = "authoredDate")]
    pub authored_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real `gh pr view --json commits` output for PR #44 — includes fields
    /// (`authors`, `committedDate`, `messageBody`) that `CommitInfo` doesn't
    /// model yet, so this checks parsing tolerates them rather than a
    /// lossless round trip.
    const PR_COMMITS_FIXTURE: &str = include_str!("fixtures/pr_commits.json");

    #[test]
    fn parses_real_pr_commits_output() {
        let commits: Vec<CommitInfo> = serde_json::from_str(PR_COMMITS_FIXTURE).unwrap();

        assert_eq!(
            commits,
            vec![
                CommitInfo {
                    oid: "edd8827c982f201326f5e0af40fb8180f317eabb".to_string(),
                    message: "feat: add doctor module".to_string(),
                    authored_at: "2026-09-14T11:41:58Z".to_string(),
                },
                CommitInfo {
                    oid: "60b4fd24f37bd8df27a8385ec665ca4faa56b00b".to_string(),
                    message: "feat: layer 1 dummy change".to_string(),
                    authored_at: "2026-09-14T23:03:34Z".to_string(),
                },
            ]
        );
    }

    #[test]
    fn round_trips_a_commit_info() {
        let commit = CommitInfo {
            oid: "abc123".to_string(),
            message: "feat: something".to_string(),
            authored_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&commit).unwrap();
        let parsed: CommitInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, commit);
    }
}
