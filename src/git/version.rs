use crate::shell::{Shell, ShellError};
use std::path::Path;

/// Returns the current git version
pub async fn version(shell: &impl Shell, repo: &Path) -> Result<String, ShellError> {
    let output = shell.run(repo, "git", &["--version"]).await?;
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{MockShell, ShellOutput};
    use std::env;

    #[tokio::test]
    async fn git_version_returns_successfully() {
        let binding = env::current_dir().unwrap();
        let repo = binding.as_path();
        let shell = MockShell::new().when(
            "git",
            &["--version"],
            Ok(ShellOutput {
                stdout: String::from("git version 4.2.0"),
                stderr: String::new(),
                exit_code: 0,
            }),
        );

        let version = version(&shell, repo).await.unwrap();

        assert_eq!(version, "git version 4.2.0")
    }
}
