#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Glyph {
    Branch,
    Current,
    Commit,
    PullRequest,
    PullRequestOpen,
    PullRequestMerged,
    PullRequestClosed,
    Check,
    Cross,
    Warning,
    Pending,
    Running,
    GitHub,
    OpenExternal,
    Up,
    Down,
}

pub const NERD_FONT: GlyphSet = GlyphSet {
    branch: "",
    current: "●",
    commit: "",

    pull_request: "",
    pull_request_open: "",
    pull_request_merged: "",
    pull_request_closed: "",

    check: "",
    cross: "",
    warning: "",
    pending: "",
    running: "",

    github: "",
    open_external: "",

    up: "",
    down: "",
};

pub const ASCII: GlyphSet = GlyphSet {
    branch: "*",
    current: ">",
    commit: "o",

    pull_request: "PR",
    pull_request_open: "PR",
    pull_request_merged: "M",
    pull_request_closed: "X",

    check: "[+]",
    cross: "[x]",
    warning: "[!]",
    pending: "[~]",
    running: "[~]",

    github: "GH",
    open_external: "->",

    up: "^",
    down: "v",
};

#[derive(Debug, Clone, Copy)]
pub struct GlyphSet {
    pub branch: &'static str,
    pub current: &'static str,
    pub commit: &'static str,

    pub pull_request: &'static str,
    pub pull_request_open: &'static str,
    pub pull_request_merged: &'static str,
    pub pull_request_closed: &'static str,

    pub check: &'static str,
    pub cross: &'static str,
    pub warning: &'static str,
    pub pending: &'static str,
    pub running: &'static str,

    pub github: &'static str,
    pub open_external: &'static str,

    pub up: &'static str,
    pub down: &'static str,
}

impl GlyphSet {
    pub fn get(&self, glyph: Glyph) -> &'static str {
        match glyph {
            Glyph::Branch => self.branch,
            Glyph::Current => self.current,
            Glyph::Commit => self.commit,
            Glyph::PullRequest => self.pull_request,
            Glyph::PullRequestOpen => self.pull_request_open,
            Glyph::PullRequestMerged => self.pull_request_merged,
            Glyph::PullRequestClosed => self.pull_request_closed,
            Glyph::Check => self.check,
            Glyph::Cross => self.cross,
            Glyph::Warning => self.warning,
            Glyph::Pending => self.pending,
            Glyph::Running => self.running,
            Glyph::GitHub => self.github,
            Glyph::OpenExternal => self.open_external,
            Glyph::Up => self.up,
            Glyph::Down => self.down,
        }
    }
}
