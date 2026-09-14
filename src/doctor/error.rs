use std::fmt;

use thiserror::Error;

use super::version::Version;

/// A single doctor check that failed.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CheckFailure {
    /// The binary could not be found on the system at all.
    #[error("{name} is not installed. Install it and make sure it's on your PATH: {install}")]
    NotInstalled {
        name: &'static str,
        install: &'static str,
    },

    /// The binary is installed but its version is too old.
    #[error("{name} {found} is too old (need {minimum}+). Upgrade it: {install}")]
    OutdatedVersion {
        name: &'static str,
        found: Version,
        minimum: Version,
        install: &'static str,
    },

    /// The binary's `--version` output couldn't be parsed.
    #[error("couldn't parse {name}'s version from `{output}`")]
    UnparseableVersion { name: &'static str, output: String },

    /// A non-version check (e.g. `gh auth status`) failed.
    #[error("{name} check failed: {reason}. {remedy}")]
    Failed {
        name: &'static str,
        reason: String,
        remedy: &'static str,
    },
}

/// The result of running `doctor::check`: every failed check, in the order
/// they were run.
#[derive(Debug, PartialEq, Eq)]
pub struct DoctorError(pub Vec<CheckFailure>);

impl fmt::Display for DoctorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = self
            .0
            .iter()
            .map(|failure| format!("- {failure}"))
            .collect::<Vec<_>>()
            .join("\n");

        write!(f, "{rendered}")
    }
}

impl std::error::Error for DoctorError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_installed_names_the_install_command() {
        let failure = CheckFailure::NotInstalled {
            name: "git",
            install: "https://git-scm.com/downloads",
        };

        assert_eq!(
            failure.to_string(),
            "git is not installed. Install it and make sure it's on your PATH: https://git-scm.com/downloads"
        );
    }

    #[test]
    fn outdated_version_shows_found_and_minimum() {
        let failure = CheckFailure::OutdatedVersion {
            name: "git",
            found: Version::new(2, 10, 0),
            minimum: Version::new(2, 20, 0),
            install: "https://git-scm.com/downloads",
        };

        assert_eq!(
            failure.to_string(),
            "git 2.10.0 is too old (need 2.20.0+). Upgrade it: https://git-scm.com/downloads"
        );
    }

    #[test]
    fn doctor_error_renders_one_line_per_failure() {
        let error = DoctorError(vec![
            CheckFailure::NotInstalled {
                name: "git",
                install: "install git",
            },
            CheckFailure::NotInstalled {
                name: "gh",
                install: "install gh",
            },
        ]);

        assert_eq!(
            error.to_string(),
            "- git is not installed. Install it and make sure it's on your PATH: install git\n\
             - gh is not installed. Install it and make sure it's on your PATH: install gh"
        );
    }
}
