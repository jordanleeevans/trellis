use std::path::Path;

use crate::shell::{ProcessShell, Shell, ShellError};

/// Returns the short-format `git status` output for the repository at `repo`.
pub async fn status(shell: &ProcessShell, repo: &Path) -> Result<String, ShellError> {
    let output = shell.run(repo, "git", &["status", "--short"]).await?;
    Ok(output.stdout)
}
