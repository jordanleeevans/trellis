pub struct Layer {
    branch: String,
    pull_request: Option<PullRequestRef>,
    commits: Vec<Commit>,
    position: usize,
}