use std::path::Path;

use crate::shell::{Shell, ShellError};

/// Returns the raw unified diff between `lower` and `upper`.
pub async fn diff(
    shell: &impl Shell,
    repo: &Path,
    lower: &str,
    upper: &str,
) -> Result<String, ShellError> {
    let range = format!("{lower}..{upper}");
    let output = shell
        .run(
            repo,
            "git",
            &["diff", "--color=never", "--find-renames", &range],
        )
        .await?;
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{MockShell, ShellOutput};

    #[tokio::test]
    async fn diffs_lower_against_upper_branch() {
        let repo = std::env::current_dir().unwrap();
        let shell = MockShell::new().when(
            "git",
            &[
                "diff",
                "--color=never",
                "--find-renames",
                "main..feature/layer-1",
            ],
            Ok(ShellOutput {
                stdout: "diff --git a/app.rs b/app.rs\n+hello".to_string(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );

        let output = diff(&shell, repo.as_path(), "main", "feature/layer-1")
            .await
            .unwrap();

        assert!(output.contains("+hello"));
    }
}
