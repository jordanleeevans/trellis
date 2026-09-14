pub struct PullRequestRef {
    number: u32,
    title: String,
    url: String,
    state: String,
    is_draft: bool,
    checks_status: String,
    review_status: String,
}