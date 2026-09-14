mod git;
mod shell;

use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let cwd = env::current_dir()?;
    let shell = shell::ProcessShell;

    let output = git::status(&shell, cwd.as_path()).await?;

    println!("{output}");

    Ok(())
}
