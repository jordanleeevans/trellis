use std::path::Path;

use crate::shell::{Shell, ShellError};

use super::error::CheckFailure;
use super::version::Version;

/// A single version-gated dependency check: run a command, parse a version
/// out of its output, and compare it against a minimum.
pub struct VersionRequirement {
    /// Human-readable name used in failure messages, e.g. `"git"`.
    pub name: &'static str,
    pub program: &'static str,
    pub args: &'static [&'static str],
    pub minimum: Version,
    /// Instructions shown when the binary is missing or outdated.
    pub install: &'static str,
}

impl VersionRequirement {
    /// Runs the check, returning `Ok(())` if `program` is installed and its
    /// version meets `minimum`.
    pub async fn check(&self, shell: &impl Shell, cwd: &Path) -> Result<(), CheckFailure> {
        let output = match shell.run(cwd, self.program, self.args).await {
            Ok(output) => output,
            Err(ShellError::BinaryNotFound(_)) => {
                return Err(CheckFailure::NotInstalled {
                    name: self.name,
                    install: self.install,
                });
            }
            Err(err) => {
                return Err(CheckFailure::Failed {
                    name: self.name,
                    reason: err.to_string(),
                    remedy: self.install,
                });
            }
        };

        let found = Version::parse(&output.stdout).map_err(|_| CheckFailure::UnparseableVersion {
            name: self.name,
            output: output.stdout.clone(),
        })?;

        if found < self.minimum {
            return Err(CheckFailure::OutdatedVersion {
                name: self.name,
                found,
                minimum: self.minimum,
                install: self.install,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{MockShell, ShellOutput};
    use std::env;

    fn requirement() -> VersionRequirement {
        VersionRequirement {
            name: "git",
            program: "git",
            args: &["--version"],
            minimum: Version::new(2, 20, 0),
            install: "https://git-scm.com/downloads",
        }
    }

    fn stdout(text: &str) -> Result<ShellOutput, ShellError> {
        Ok(ShellOutput {
            stdout: text.to_string(),
            stderr: String::new(),
            exit_code: 0,
        })
    }

    #[tokio::test]
    async fn passes_when_version_meets_minimum() {
        let cwd = env::current_dir().unwrap();
        let shell = MockShell::new().when("git", &["--version"], stdout("git version 2.43.0"));

        let result = requirement().check(&shell, cwd.as_path()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn fails_when_version_is_below_minimum() {
        let cwd = env::current_dir().unwrap();
        let shell = MockShell::new().when("git", &["--version"], stdout("git version 2.10.0"));

        let result = requirement().check(&shell, cwd.as_path()).await;

        assert_eq!(
            result,
            Err(CheckFailure::OutdatedVersion {
                name: "git",
                found: Version::new(2, 10, 0),
                minimum: Version::new(2, 20, 0),
                install: "https://git-scm.com/downloads",
            })
        );
    }

    #[tokio::test]
    async fn fails_when_binary_is_missing() {
        let cwd = env::current_dir().unwrap();
        let shell = MockShell::new().when(
            "git",
            &["--version"],
            Err(ShellError::BinaryNotFound("git".to_string())),
        );

        let result = requirement().check(&shell, cwd.as_path()).await;

        assert_eq!(
            result,
            Err(CheckFailure::NotInstalled {
                name: "git",
                install: "https://git-scm.com/downloads",
            })
        );
    }

    #[tokio::test]
    async fn fails_when_version_cannot_be_parsed() {
        let cwd = env::current_dir().unwrap();
        let shell = MockShell::new().when("git", &["--version"], stdout("not a version"));

        let result = requirement().check(&shell, cwd.as_path()).await;

        assert_eq!(
            result,
            Err(CheckFailure::UnparseableVersion {
                name: "git",
                output: "not a version".to_string(),
            })
        );
    }
}
