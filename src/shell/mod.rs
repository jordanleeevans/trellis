mod error;
mod output;
mod process;

pub use error::ShellError;
pub use output::ShellOutput;

use std::path::Path;

#[async_trait::async_trait]
pub trait Shell: Send + Sync {
    async fn run(
        &self,
        cwd: &Path,
        program: &str,
        args: &[&str],
    ) -> Result<ShellOutput, ShellError>;
}

