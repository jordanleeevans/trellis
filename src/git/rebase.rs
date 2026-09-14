use std::path::Path;

use crate::shell::{ProcessShell, Shell, ShellError};

/// Rebases the current branch of the repository at `repo` onto `onto`.
pub async fn rebase(shell: &ProcessShell, repo: &Path, onto: &str) -> Result<String, ShellError> {
    let output = shell.run(repo, "git", &["rebase", onto]).await?;
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn run(shell: &ProcessShell, repo: &Path, args: &[&str]) {
        shell.run(repo, "git", args).await.unwrap();
    }

    #[tokio::test]
    async fn rebases_feature_branch_onto_main() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();
        let shell = ProcessShell;

        run(&shell, repo, &["init", "-b", "main"]).await;
        run(&shell, repo, &["config", "user.email", "test@example.com"]).await;
        run(&shell, repo, &["config", "user.name", "Test"]).await;
        run(
            &shell,
            repo,
            &["commit", "--allow-empty", "-m", "main commit"],
        )
        .await;
        run(&shell, repo, &["checkout", "-b", "feature"]).await;
        run(
            &shell,
            repo,
            &["commit", "--allow-empty", "-m", "feature commit"],
        )
        .await;
        run(&shell, repo, &["checkout", "main"]).await;
        run(
            &shell,
            repo,
            &["commit", "--allow-empty", "-m", "another main commit"],
        )
        .await;
        run(&shell, repo, &["checkout", "feature"]).await;

        let result = rebase(&shell, repo, "main").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn errors_when_target_does_not_exist() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();
        let shell = ProcessShell;

        run(&shell, repo, &["init", "-b", "main"]).await;
        run(&shell, repo, &["config", "user.email", "test@example.com"]).await;
        run(&shell, repo, &["config", "user.name", "Test"]).await;
        run(
            &shell,
            repo,
            &["commit", "--allow-empty", "-m", "main commit"],
        )
        .await;

        let result = rebase(&shell, repo, "does-not-exist").await;

        assert!(result.is_err());
    }
}
