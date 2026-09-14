use std::path::Path;

use crate::shell::{Shell, ShellError};

use super::error::CheckFailure;

const NAME: &str = "gh auth";
const INSTALL: &str = "https://cli.github.com";
const REMEDY: &str = "run `gh auth login`";

/// Checks that `gh` is authenticated (`gh auth status` exits successfully).
pub async fn check(shell: &impl Shell, cwd: &Path) -> Result<(), CheckFailure> {
    match shell.run(cwd, "gh", &["auth", "status"]).await {
        Ok(_) => Ok(()),
        Err(ShellError::BinaryNotFound(_)) => Err(CheckFailure::NotInstalled {
            name: "gh",
            install: INSTALL,
        }),
        Err(err) => Err(CheckFailure::Failed {
            name: NAME,
            reason: err.to_string(),
            remedy: REMEDY,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::MockShell;
    use crate::shell::ShellOutput;
    use std::env;

    #[tokio::test]
    async fn passes_when_authenticated() {
        let cwd = env::current_dir().unwrap();
        let shell = MockShell::new().when(
            "gh",
            &["auth", "status"],
            Ok(ShellOutput {
                stdout: "Logged in to github.com as octocat".to_string(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );

        assert!(check(&shell, cwd.as_path()).await.is_ok());
    }

    #[tokio::test]
    async fn fails_with_login_remedy_when_not_authenticated() {
        let cwd = env::current_dir().unwrap();
        let command_failed = ShellError::CommandFailed {
            program: "gh".to_string(),
            output: ShellOutput {
                stdout: String::new(),
                stderr: "You are not logged into any GitHub hosts".to_string(),
                exit_code: 1,
            },
        };
        let expected_reason = command_failed.to_string();
        let shell = MockShell::new().when("gh", &["auth", "status"], Err(command_failed));

        let result = check(&shell, cwd.as_path()).await;

        assert_eq!(
            result,
            Err(CheckFailure::Failed {
                name: NAME,
                reason: expected_reason,
                remedy: REMEDY,
            })
        );
    }

    #[tokio::test]
    async fn fails_when_gh_is_missing() {
        let cwd = env::current_dir().unwrap();
        let shell = MockShell::new().when(
            "gh",
            &["auth", "status"],
            Err(ShellError::BinaryNotFound("gh".to_string())),
        );

        let result = check(&shell, cwd.as_path()).await;

        assert_eq!(
            result,
            Err(CheckFailure::NotInstalled {
                name: "gh",
                install: INSTALL,
            })
        );
    }
}
