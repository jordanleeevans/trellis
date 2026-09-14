//! Checks the local environment has the tools trellis depends on.

mod error;
mod gh;
mod gh_auth;
mod gh_stack;
mod git;
mod requirement;
mod version;

use std::path::Path;

use crate::shell::Shell;

pub use error::DoctorError;

/// Runs every doctor check and returns `Ok(())` if all pass, or a
/// [`DoctorError`] listing every check that failed.
pub async fn check(shell: &impl Shell, cwd: &Path) -> Result<(), DoctorError> {
    let mut failures = Vec::new();

    for result in [
        git::check(shell, cwd).await,
        gh::check(shell, cwd).await,
        gh_auth::check(shell, cwd).await,
        gh_stack::check(shell, cwd).await,
    ] {
        if let Err(failure) = result {
            failures.push(failure);
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(DoctorError(failures))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{MockShell, ShellOutput};
    use std::env;

    fn ok(stdout: &str) -> Result<ShellOutput, crate::shell::ShellError> {
        Ok(ShellOutput {
            stdout: stdout.to_string(),
            stderr: String::new(),
            exit_code: 0,
        })
    }

    #[tokio::test]
    async fn passes_when_every_check_passes() {
        let cwd = env::current_dir().unwrap();
        let shell = MockShell::new()
            .when("git", &["--version"], ok("git version 2.43.0"))
            .when("gh", &["--version"], ok("gh version 2.90.0 (2024-01-01)"))
            .when("gh", &["auth", "status"], ok("Logged in to github.com"))
            .when(
                "gh",
                &["extension", "list"],
                ok("NAME       REPO                     VERSION\ngh stack   timothyandrew/gh-stack  v0.5.0"),
            );

        assert!(check(&shell, cwd.as_path()).await.is_ok());
    }

    #[tokio::test]
    async fn collects_every_failing_check() {
        let cwd = env::current_dir().unwrap();
        let shell = MockShell::new()
            .when("git", &["--version"], ok("git version 2.10.0"))
            .when("gh", &["--version"], ok("gh version 2.90.0 (2024-01-01)"))
            .when("gh", &["auth", "status"], ok("Logged in to github.com"))
            .when(
                "gh",
                &["extension", "list"],
                ok("NAME       REPO                     VERSION\ngh stack   timothyandrew/gh-stack  v0.5.0"),
            );

        let result = check(&shell, cwd.as_path()).await;

        let err = result.unwrap_err();
        assert_eq!(err.0.len(), 1);
    }
}
