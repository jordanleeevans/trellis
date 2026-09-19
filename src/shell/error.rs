// src/shell/error.rs

use thiserror::Error;

use super::ShellOutput;

/// Errors that can occur while running an external command.
#[derive(Debug, Error)]
pub enum ShellError {
    /// The command ran but exited with a non-zero status.
    #[error("command failed: {program} exited with code {output:?}")]
    CommandFailed {
        program: String,
        output: ShellOutput,
    },

    /// The requested binary could not be found on the system.
    #[error("binary not found: {0}")]
    BinaryNotFound(String),

    /// The process failed to spawn for a reason other than a missing binary.
    #[error("failed to start process: {0}")]
    Spawn(#[source] std::io::Error),

    /// An I/O error occurred while reading the process's output.
    #[error("failed while reading process output: {0}")]
    Io(#[from] std::io::Error),

    /// The process exited without reporting an exit code (e.g. killed by a signal).
    #[error("process terminated without an exit code")]
    MissingExitCode,

    /// The process produced output that could not be interpreted as expected.
    #[error("unexpected output: {0}")]
    UnexpectedOutput(String),

    /// The process did not complete before the configured timeout.
    #[error("command timed out after {timeout:?}: {program}")]
    Timeout {
        program: String,
        timeout: std::time::Duration,
    },
}
