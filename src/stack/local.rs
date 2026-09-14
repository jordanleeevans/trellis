use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Errors that can occur while reading the local `gh stack` metadata file.
#[derive(Debug, Error)]
pub enum LocalStackError {
    /// The metadata file exists but couldn't be read.
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The metadata file exists but isn't valid JSON in the expected shape.
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

/// One stack as tracked locally, straight from `.git/gh-stack`.
///
/// This is distinct from [`super::Stack`] (which mirrors `gh stack view
/// --json` for a single, already-checked-out stack): `gh stack` has no
/// command to list every locally tracked stack, so this reads its local
/// metadata file directly to enumerate them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LocalStack {
    pub trunk: LocalTrunk,
    pub branches: Vec<LocalBranch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LocalTrunk {
    pub branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LocalBranch {
    pub branch: String,
    #[serde(default)]
    pub base: String,
}

#[derive(Debug, Default, Deserialize)]
struct StackFile {
    #[serde(default)]
    stacks: Vec<LocalStack>,
}

/// Reads every stack tracked locally in the repository at `repo`, from
/// `.git/gh-stack`.
///
/// Returns an empty list (not an error) if the file doesn't exist, which is
/// the normal case for a repository with no stacks initialized yet.
pub fn read_local_stacks(repo: &Path) -> Result<Vec<LocalStack>, LocalStackError> {
    let path = repo.join(".git").join("gh-stack");

    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(LocalStackError::Read { path, source: error }),
    };

    let file: StackFile =
        serde_json::from_str(&contents).map_err(|error| LocalStackError::Parse {
            path: path.clone(),
            source: error,
        })?;

    Ok(file.stacks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_empty_when_the_metadata_file_is_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp_dir.path().join(".git")).unwrap();

        let stacks = read_local_stacks(temp_dir.path()).unwrap();

        assert!(stacks.is_empty());
    }

    #[test]
    fn parses_multiple_locally_tracked_stacks() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp_dir.path().join(".git")).unwrap();
        std::fs::write(
            temp_dir.path().join(".git").join("gh-stack"),
            r#"{
                "schemaVersion": 1,
                "repository": "",
                "stacks": [
                    {
                        "trunk": { "branch": "main", "head": "abc123" },
                        "branches": [
                            { "branch": "layer-1", "base": "abc123" },
                            { "branch": "layer-2", "base": "abc123" }
                        ]
                    },
                    {
                        "trunk": { "branch": "main", "head": "abc123" },
                        "branches": [
                            { "branch": "other-a", "base": "abc123" }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

        let stacks = read_local_stacks(temp_dir.path()).unwrap();

        assert_eq!(stacks.len(), 2);
        assert_eq!(stacks[0].trunk.branch, "main");
        assert_eq!(stacks[0].branches.len(), 2);
        assert_eq!(stacks[0].branches[0].branch, "layer-1");
        assert_eq!(stacks[1].branches[0].branch, "other-a");
    }

    #[test]
    fn errors_on_invalid_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp_dir.path().join(".git")).unwrap();
        std::fs::write(temp_dir.path().join(".git").join("gh-stack"), "not json").unwrap();

        let result = read_local_stacks(temp_dir.path());

        assert!(matches!(result, Err(LocalStackError::Parse { .. })));
    }
}
