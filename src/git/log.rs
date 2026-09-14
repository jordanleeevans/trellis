use std::path::Path;

use crate::shell::{Shell, ShellError};

/// Returns the one-line-per-commit `git log` output for the repository at `repo`.
pub async fn log(shell: &impl Shell, repo: &Path) -> Result<String, ShellError> {
    let output = shell.run(repo, "git", &["log", "--oneline"]).await?;
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{MockShell, ProcessShell, ShellOutput};

    #[tokio::test]
    async fn returns_shell_stdout() {
        let repo = std::env::current_dir().unwrap();
        let shell = MockShell::new().when(
            "git",
            &["log", "--oneline"],
            Ok(ShellOutput {
                stdout: "abc1234 initial commit".to_string(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );

        let output = log(&shell, repo.as_path()).await.unwrap();

        assert_eq!(output, "abc1234 initial commit");
    }

    async fn init_repo_with_commit(shell: &ProcessShell, repo: &Path, message: &str) {
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
            .run(repo, "git", &["commit", "--allow-empty", "-m", message])
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn returns_commit_messages() {
        let temp_dir = tempfile::tempdir().unwrap();
        let shell = ProcessShell;

        init_repo_with_commit(&shell, temp_dir.path(), "initial commit").await;

        let output = log(&shell, temp_dir.path()).await.unwrap();

        assert!(output.contains("initial commit"));
    }

    #[tokio::test]
    async fn errors_for_repo_with_no_commits() {
        let temp_dir = tempfile::tempdir().unwrap();
        let shell = ProcessShell;

        shell.run(temp_dir.path(), "git", &["init"]).await.unwrap();

        let result = log(&shell, temp_dir.path()).await;

        assert!(result.is_err());
    }
}
