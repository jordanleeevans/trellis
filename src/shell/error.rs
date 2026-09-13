// src/shell/error.rs

use thiserror::Error;

use super::ShellOutput;

#[derive(Debug, Error)]
pub enum ShellError {
    #[error("command failed: {program} exited with code {output:?}")]
    CommandFailed {
        program: String,
        output: ShellOutput,
    },

    #[error("binary not found: {0}")]
    BinaryNotFound(String),

    #[error("failed to start process: {0}")]
    Spawn(#[source] std::io::Error),

    #[error("failed while reading process output: {0}")]
    Io(#[from] std::io::Error),

    #[error("process terminated without an exit code")]
    MissingExitCode,

    #[error("unexpected output: {0}")]
    UnexpectedOutput(String),
}
