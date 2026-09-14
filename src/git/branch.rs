use std::path::Path;

use crate::shell::{ProcessShell, Shell, ShellError};

/// Returns the `git branch --list` output for the repository at `repo`.
pub async fn branch(shell: &ProcessShell, repo: &Path) -> Result<String, ShellError> {
    let output = shell.run(repo, "git", &["branch", "--list"]).await?;
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn init_repo_with_commit(shell: &ProcessShell, repo: &Path) {
        shell.run(repo, "git", &["init"]).await.unwrap();
        shell
            .run(repo, "git", &["config", "user.email", "test@example.com"])
            .await
            .unwrap();
        shell
            .run(repo, "git", &["config", "user.name", "Test"])
            .await
            .unwrap();
        shell
            .run(
                repo,
                "git",
                &["commit", "--allow-empty", "-m", "initial commit"],
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn lists_created_branches() {
        let temp_dir = tempfile::tempdir().unwrap();
        let shell = ProcessShell;

        init_repo_with_commit(&shell, temp_dir.path()).await;
        shell
            .run(temp_dir.path(), "git", &["branch", "feature-a"])
            .await
            .unwrap();

        let output = branch(&shell, temp_dir.path()).await.unwrap();

        assert!(output.contains("feature-a"));
    }

    #[tokio::test]
    async fn errors_for_directory_that_is_not_a_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let shell = ProcessShell;

        let result = branch(&shell, temp_dir.path()).await;

        assert!(result.is_err());
    }
}
