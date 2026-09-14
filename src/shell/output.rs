/// The captured result of running a command to completion.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl ShellOutput {
    /// Returns `true` if the command exited with code `0`.
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_returns_true_when_exit_code_is_zero() {
        let output = ShellOutput {
            stdout: String::from("stdout"),
            stderr: String::from("stderr"),
            exit_code: 0,
        };

        assert!(output.success());
    }

    #[test]
    fn success_returns_false_when_exit_code_is_non_zero() {
        let output = ShellOutput {
            stdout: String::from("stdout"),
            stderr: String::from("stderr"),
            exit_code: 1,
        };

        assert_ne!(output.exit_code, 0);
        assert_eq!(output.success(), false);
    }
}
