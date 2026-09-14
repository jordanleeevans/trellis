use std::path::Path;

use std::process::Stdio;

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc,
};

use super::{Shell, ShellError, ShellOutput};

/// A [`Shell`] implementation that runs commands as native OS processes.
#[derive(Debug, Default)]
pub struct ProcessShell;

/// An event emitted while a [`RunningCommand`] executes.
#[derive(Debug, Clone, PartialEq)]
pub enum ShellEvent {
    /// A line written to the process's stdout.
    Stdout(String),
    /// A line written to the process's stderr.
    Stderr(String),
    /// The process has exited; no further events follow.
    Finished(ShellOutput),
}

/// A handle to a command that was started with [`ProcessShell::stream`].
pub struct RunningCommand {
    /// Receives [`ShellEvent`]s as the command produces output and exits.
    pub events: mpsc::UnboundedReceiver<ShellEvent>,
}

#[async_trait::async_trait]
impl Shell for ProcessShell {
    /// Runs `program` to completion, buffering its stdout and stderr.
    async fn run(
        &self,
        cwd: &Path,
        program: &str,
        args: &[&str],
    ) -> Result<ShellOutput, ShellError> {
        let result = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .output()
            .await;

        let output = match result {
            Ok(output) => output,

            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(ShellError::BinaryNotFound(program.to_owned()));
            }

            Err(error) => {
                return Err(ShellError::Spawn(error));
            }
        };

        let result = ShellOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().ok_or(ShellError::MissingExitCode)?,
        };

        if result.success() {
            Ok(result)
        } else {
            Err(ShellError::CommandFailed {
                program: program.to_owned(),
                output: result,
            })
        }
    }
}

impl ProcessShell {
    /// Spawns `program` and streams its stdout/stderr lines as they arrive,
    /// rather than waiting for it to finish like [`Shell::run`].
    pub fn stream(
        &self,
        cwd: &Path,
        program: &str,
        args: &[&str],
    ) -> Result<RunningCommand, ShellError> {
        let mut child = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    ShellError::BinaryNotFound(program.to_owned())
                } else {
                    ShellError::Spawn(error)
                }
            })?;

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut stdout = BufReader::new(stdout).lines();
            let mut stderr = BufReader::new(stderr).lines();

            loop {
                tokio::select! {
                    line = stdout.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                let _ = tx.send(ShellEvent::Stdout(line));
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }

                    line = stderr.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                let _ = tx.send(ShellEvent::Stderr(line));
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                }
            }

            if let Ok(status) = child.wait().await {
                let _ = tx.send(ShellEvent::Finished(ShellOutput {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: status.code().unwrap_or(-1),
                }));
            }
        });

        Ok(RunningCommand { events: rx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn stream_returns_binary_not_found_for_missing_program() {
        let result = ProcessShell.stream(
            env::current_dir().unwrap().as_path(),
            "non_existent_program",
            &[],
        );

        match result {
            Err(ShellError::BinaryNotFound(program)) => {
                assert_eq!(program, "non_existent_program");
            }
            _ => panic!("expected BinaryNotFound"),
        }
    }

    #[tokio::test]
    async fn returns_stdout_for_successful_command() {
        let result = ProcessShell
            .run(env::current_dir().unwrap().as_path(), "git", &["--version"])
            .await
            .unwrap();

        assert!(result.stdout.starts_with("git version"));
        assert_eq!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn returns_command_failed_for_non_zero_exit_code() {
        let result = ProcessShell
            .run(
                env::current_dir().unwrap().as_path(),
                "git",
                &["not-a-real-git-command"],
            )
            .await;

        match result {
            Err(ShellError::CommandFailed { program, output }) => {
                assert_eq!(program, "git");
                assert_ne!(output.exit_code, 0);
                assert!(!output.stderr.is_empty());
            }
            other => panic!("expected CommandFailed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn runs_command_in_given_working_directory() {
        let temp_dir = tempfile::tempdir().unwrap();

        let result = ProcessShell
            .run(temp_dir.path(), "git", &["init"])
            .await
            .unwrap();

        assert!(result.success());
        assert!(temp_dir.path().join(".git").exists());
    }

    #[tokio::test]
    async fn test_stream_returns_stdout_and_finished_event() {
        let running_cmd = ProcessShell
            .stream(env::current_dir().unwrap().as_path(), "echo", &["hello"])
            .unwrap();

        let mut events = running_cmd.events;

        assert_eq!(
            Some(ShellEvent::Stdout("hello".to_string())),
            events.recv().await
        );

        assert_eq!(
            events.recv().await,
            Some(ShellEvent::Finished(ShellOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 0
            }))
        );
    }
    #[tokio::test]
    async fn stream_returns_stderr() {
        let running_cmd = ProcessShell
            .stream(
                env::current_dir().unwrap().as_path(),
                "ls",
                &["--definitely-invalid-option"],
            )
            .unwrap();

        let mut events = running_cmd.events;

        match events.recv().await {
            Some(ShellEvent::Stderr(message)) => {
                assert!(!message.is_empty());
            }
            other => panic!("expected stderr event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn stream_returns_non_zero_exit_code() {
        let running_cmd = ProcessShell
            .stream(
                env::current_dir().unwrap().as_path(),
                "sh",
                &["-c", "exit 42"],
            )
            .unwrap();

        let mut events = running_cmd.events;

        assert_eq!(
            events.recv().await,
            Some(ShellEvent::Finished(ShellOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 42,
            }))
        );
    }

    #[tokio::test]
    async fn stream_emits_stdout_and_stderr() {
        let running_cmd = ProcessShell
            .stream(
                env::current_dir().unwrap().as_path(),
                "sh",
                &["-c", "echo stdout-message; echo stderr-message >&2"],
            )
            .unwrap();

        let mut events = running_cmd.events;

        let mut saw_stdout = false;
        let mut saw_stderr = false;
        let mut saw_finished = false;

        while let Some(event) = events.recv().await {
            match event {
                ShellEvent::Stdout(line) => {
                    if line == "stdout-message" {
                        saw_stdout = true;
                    }
                }

                ShellEvent::Stderr(line) => {
                    if line == "stderr-message" {
                        saw_stderr = true;
                    }
                }

                ShellEvent::Finished(output) => {
                    assert_eq!(output.exit_code, 0);
                    saw_finished = true;
                    break;
                }
            }
        }

        assert!(saw_stdout);
        assert!(saw_stderr);
        assert!(saw_finished);
    }

    #[tokio::test]
    async fn slow_command_does_not_block_stream_or_event_polling() {
        let spawn_start = Instant::now();

        let running_cmd = ProcessShell
            .stream(
                env::current_dir().unwrap().as_path(),
                "sh",
                &["-c", "sleep 0.3; echo done"],
            )
            .unwrap();

        // stream() must return immediately, well before the command's sleep
        // finishes, instead of waiting for the process to exit.
        assert!(
            spawn_start.elapsed() < Duration::from_millis(100),
            "stream() blocked the caller instead of returning immediately"
        );

        let mut events = running_cmd.events;

        // Polling for events while the command is still sleeping must not
        // block either, the way a render loop's per-frame poll would call it.
        let poll_start = Instant::now();
        assert!(
            events.try_recv().is_err(),
            "expected no events yet while the command is still sleeping"
        );
        assert!(
            poll_start.elapsed() < Duration::from_millis(50),
            "polling for events blocked instead of returning immediately"
        );

        let mut saw_stdout = false;
        let mut saw_finished = false;

        while let Some(event) = events.recv().await {
            match event {
                ShellEvent::Stdout(line) => {
                    if line == "done" {
                        saw_stdout = true;
                    }
                }
                ShellEvent::Finished(output) => {
                    assert_eq!(output.exit_code, 0);
                    saw_finished = true;
                    break;
                }
                ShellEvent::Stderr(_) => {}
            }
        }

        assert!(saw_stdout);
        assert!(saw_finished);
    }
}
