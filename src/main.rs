mod git;
mod shell;

use crate::git::status::status as git_status;
use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let cwd = env::current_dir()?;
    let shell = shell::ProcessShell;

    let output = git_status(&shell, cwd.as_path()).await?;

    println!("{output}");

    Ok(())
}
